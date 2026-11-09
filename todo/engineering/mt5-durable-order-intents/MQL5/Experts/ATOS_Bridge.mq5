//+------------------------------------------------------------------+
//| ATOS_Bridge.mq5                                                  |
//| Authenticated MT5 telemetry and market-data ingestion adapter.  |
//| READ-ONLY: no order placement, modification, or deletion APIs.   |
//+------------------------------------------------------------------+
#property strict
#property version   "6.00"
#property description "ATOS localhost MT5 telemetry and durable market-data adapter; read-only"

input string InpBridgeHost = "127.0.0.1";
input int    InpBridgePort = 47650;
input string InpPairingToken = "";
input string InpWatchedSymbol = ""; // Primary symbol; empty means the chart symbol.
input string InpWatchSymbols = "";  // Additional broker-native symbols, comma-separated.
input string InpTimeframes = "M1,M5,M15,H1";
input int    InpHeartbeatSeconds = 1;
input int    InpConnectTimeoutMs = 1500;
input int    InpResponseTimeoutMs = 3000;
input int    InpSessionCalendarRefreshSeconds = 300;

#define ATOS_PROTOCOL_VERSION 6
#define ATOS_MARGIN_PREFLIGHT_MAX_AGE_MS 5000
#define ATOS_MAX_POSITIONS 200
#define ATOS_MAX_PENDING_ORDERS 200
#define ATOS_MAX_FRAME_BYTES 2097152
#define ATOS_MAX_SYMBOLS 8
#define ATOS_MAX_TIMEFRAMES 6
#define ATOS_TICKS_PER_SYMBOL 512
#define ATOS_CLOSED_BARS_PER_SERIES 64
#define ATOS_BAR_AUDIT_LOOKBACK 3
#define ATOS_BAR_AUDIT_INTERVAL_SECONDS 60

int      g_socket = INVALID_HANDLE;
ulong    g_sequence = 0;
string   g_client_id = "";
string   g_account_login = "";
string   g_account_server = "";
string   g_account_currency = "";
string   g_session_id = "";
string   g_symbol = "";
bool     g_authenticated = false;
bool     g_session_calendar_needs_send = true;
uint     g_last_connect_attempt = 0;
string   g_symbols[];
ENUM_TIMEFRAMES g_timeframes[];
long     g_last_tick_time[];
string   g_last_tick_signature[];
long     g_last_bar_time[];
long     g_last_bar_audit_server_time=0;
string   g_session_schedules_cache="";
uint     g_last_session_calendar_refresh=0;
string   g_margin_preflight_results_json="[]";
string   g_margin_preflight_request_json="";
ulong    g_snapshot_position_tickets[];
string   g_snapshot_position_symbols[];
long     g_snapshot_position_types[];
double   g_snapshot_position_volumes[];
double   g_snapshot_position_prices[];
double   g_snapshot_position_stop_losses[];
ulong    g_snapshot_order_tickets[];
string   g_snapshot_order_symbols[];
long     g_snapshot_order_types[];
long     g_snapshot_order_states[];
double   g_snapshot_order_initial_volumes[];
double   g_snapshot_order_current_volumes[];
double   g_snapshot_order_open_prices[];
double   g_snapshot_order_stop_limit_prices[];
double   g_snapshot_order_stop_losses[];
double   g_snapshot_account_balance=0.0;
double   g_snapshot_account_equity=0.0;
double   g_snapshot_account_credit=0.0;
double   g_snapshot_account_profit=0.0;
double   g_snapshot_account_margin=0.0;
double   g_snapshot_account_free_margin=0.0;
double   g_snapshot_account_margin_level=0.0;
double   g_snapshot_account_margin_initial=0.0;
double   g_snapshot_account_margin_maintenance=0.0;
double   g_snapshot_account_margin_call_level=0.0;
double   g_snapshot_account_stop_out_level=0.0;
long     g_snapshot_account_margin_stopout_mode=0;
long     g_snapshot_account_margin_mode=0;
long     g_snapshot_account_limit_orders=0;
long     g_snapshot_account_leverage=0;
bool     g_snapshot_account_hedge_allowed=false;
bool     g_snapshot_account_trade_allowed=false;
bool     g_snapshot_terminal_trade_allowed=false;
bool     g_snapshot_algo_trading_allowed=false;

string JsonEscape(const string value)
  {
   string result="";
   for(int i=0;i<StringLen(value);i++)
     {
      ushort character=StringGetCharacter(value,i);
      if(character==92) result+="\\\\";
      else if(character==34) result+="\\\"";
      else if(character==8) result+="\\b";
      else if(character==9) result+="\\t";
      else if(character==10) result+="\\n";
      else if(character==12) result+="\\f";
      else if(character==13) result+="\\r";
      else if(character<32) result+=StringFormat("\\u%04X",(int)character);
      else result+=StringSubstr(value,i,1);
     }
   return result;
  }

string Number(const double value,const int digits=16)
  {
   if(!MathIsValidNumber(value))
      return "null";
   return DoubleToString(value,digits);
  }

bool SendLine(const string json)
  {
   if(g_socket==INVALID_HANDLE || !SocketIsConnected(g_socket))
      return false;
   uchar bytes[];
   int length=StringToCharArray(json+"\n",bytes,0,WHOLE_ARRAY,CP_UTF8)-1;
   if(length<=0 || length>ATOS_MAX_FRAME_BYTES)
      return false;
   int sent_total=0;
   while(sent_total<length && !IsStopped())
     {
      uchar remaining[];
      int remaining_count=length-sent_total;
      if(ArrayResize(remaining,remaining_count)!=remaining_count)
         return false;
      ArrayCopy(remaining,bytes,0,sent_total,remaining_count);
      int sent=SocketSend(g_socket,remaining,(uint)remaining_count);
      if(sent<=0)
         return false;
      sent_total+=sent;
     }
   return sent_total==length;
  }

bool ReadLine(string &response,const uint timeout_ms)
  {
   response="";
   uint started=GetTickCount();
   while(!IsStopped() && (uint)(GetTickCount()-started)<timeout_ms)
     {
      uint available=SocketIsReadable(g_socket);
      if(available>0)
        {
         uchar buffer[];
         int request=(int)MathMin((double)available,8192.0);
         int received=SocketRead(g_socket,buffer,request,timeout_ms);
         if(received<=0)
            return false;
         response+=CharArrayToString(buffer,0,received,CP_UTF8);
         if(StringLen(response)>ATOS_MAX_FRAME_BYTES)
            return false;
         int newline=StringFind(response,"\n");
         if(newline>=0)
           {
            response=StringSubstr(response,0,newline);
            return StringLen(response)>0;
           }
        }
      Sleep(10);
     }
   return false;
  }

string StringArrayJson(const string &values[])
  {
   string result="[";
   for(int i=0;i<ArraySize(values);i++)
     {
      if(i>0) result+=",";
      result+="\""+JsonEscape(values[i])+"\"";
     }
   return result+"]";
  }

bool ReadMapCursor(const string json,const string map_name,const string key,long &value)
  {
   string map_marker="\""+map_name+"\":{";
   int map_position=StringFind(json,map_marker);
   if(map_position<0)
      return false;
   string tail=StringSubstr(json,map_position+StringLen(map_marker));
   int map_end=StringFind(tail,"}");
   if(map_end<0)
      return false;
   string map_contents=StringSubstr(tail,0,map_end);
   string key_marker="\""+key+"\":";
   int key_position=StringFind(map_contents,key_marker);
   if(key_position<0)
      return false;
   string number=StringSubstr(map_contents,key_position+StringLen(key_marker));
   int comma=StringFind(number,",");
   if(comma>=0)
      number=StringSubstr(number,0,comma);
   StringTrimLeft(number);
   StringTrimRight(number);
   if(StringLen(number)==0)
      return false;
   for(int i=0;i<StringLen(number);i++)
     {
      ushort character=StringGetCharacter(number,i);
      if(character<'0' || character>'9')
         return false;
     }
   value=StringToInteger(number);
   return value>=0;
  }

bool ReadUnsignedField(const string json,const string field,ulong &value)
  {
   string key="\""+field+"\":";
   int position=StringFind(json,key);
   if(position<0)
      return false;
   string tail=StringSubstr(json,position+StringLen(key));
   int comma=StringFind(tail,",");
   int brace=StringFind(tail,"}");
   int end=StringLen(tail);
   if(comma>=0 && comma<end) end=comma;
   if(brace>=0 && brace<end) end=brace;
   string number=StringSubstr(tail,0,end);
   StringTrimLeft(number);
   StringTrimRight(number);
   if(StringLen(number)==0)
      return false;
   long parsed=StringToInteger(number);
   if(parsed<0)
      return false;
   value=(ulong)parsed;
   return true;
  }

bool ReadObjectField(const string json,const string field,string &object)
  {
   object="";
   string marker="\""+field+"\":";
   int position=StringFind(json,marker);
   if(position<0)
      return false;
   string tail=StringSubstr(json,position+StringLen(marker));
   StringTrimLeft(tail);
   if(StringLen(tail)==0 || StringGetCharacter(tail,0)!='{')
      return false;
   int end=StringFind(tail,"}");
   if(end<0)
      return false;
   object=StringSubstr(tail,1,end-1);
   return StringLen(object)>0;
  }

bool ReadObjectString(const string object,const string field,string &value)
  {
   value="";
   string marker="\""+field+"\":";
   int position=StringFind(object,marker);
   if(position<0)
      return false;
   string tail=StringSubstr(object,position+StringLen(marker));
   StringTrimLeft(tail);
   if(StringLen(tail)<2 || StringGetCharacter(tail,0)!='\"')
      return false;
   int end=StringFind(tail,"\"",1);
   if(end<1)
      return false;
   value=StringSubstr(tail,1,end-1);
   return StringLen(value)>0;
  }

bool ReadObjectNumber(const string object,const string field,double &value)
  {
   value=0.0;
   string marker="\""+field+"\":";
   int position=StringFind(object,marker);
   if(position<0)
      return false;
   string tail=StringSubstr(object,position+StringLen(marker));
   int comma=StringFind(tail,",");
   int brace=StringFind(tail,"}");
   int end=StringLen(tail);
   if(comma>=0 && comma<end) end=comma;
   if(brace>=0 && brace<end) end=brace;
   string number=StringSubstr(tail,0,end);
   StringTrimLeft(number);
   StringTrimRight(number);
   if(StringLen(number)==0)
      return false;
   value=StringToDouble(number);
   return MathIsValidNumber(value);
  }

bool ReadObjectInteger(const string object,const string field,long &value)
  {
   double parsed=0.0;
   if(!ReadObjectNumber(object,field,parsed) || parsed<0.0 || parsed>9223372036854775807.0
      || MathAbs(parsed-MathRound(parsed))>0.0)
      return false;
   value=(long)parsed;
   return true;
  }

bool IsWatchedSymbol(const string symbol)
  {
   for(int index=0;index<ArraySize(g_symbols);index++)
      if(g_symbols[index]==symbol)
         return true;
   return false;
  }

string BuildMarginPreflightResult(const string request_object,const MqlTick &snapshot_tick)
  {
   string request_id,symbol,side;
   double volume=0.0,stop_loss=0.0;
   long expires_at_ms=0;
   if(!ReadObjectString(request_object,"requestId",request_id)
      || !ReadObjectString(request_object,"symbol",symbol)
      || !ReadObjectString(request_object,"side",side)
      || !ReadObjectNumber(request_object,"volume",volume)
      || !ReadObjectNumber(request_object,"stopLossPrice",stop_loss)
      || !ReadObjectInteger(request_object,"expiresAtMs",expires_at_ms)
      || StringLen(request_id)!=36 || symbol!=g_symbol || !IsWatchedSymbol(symbol)
      || (side!="BUY" && side!="SELL") || volume<=0.0 || stop_loss<=0.0)
      return "";

   long calculated_at_ms=(long)TimeGMT()*1000;
   MqlTick tick=snapshot_tick;
   bool tick_ok=tick.time_msc>0 && tick.bid>0.0 && tick.ask>=tick.bid;
   double price=0.0;
   if(tick_ok)
      price=(side=="BUY" ? tick.ask : tick.bid);
   int error_code=0;
   double margin=0.0;
   double profit_at_stop=0.0;
   bool success=false;
   if(expires_at_ms<calculated_at_ms || expires_at_ms-calculated_at_ms>ATOS_MARGIN_PREFLIGHT_MAX_AGE_MS)
      error_code=-1001;
   else if(!tick_ok || MathAbs(calculated_at_ms-tick.time_msc)>ATOS_MARGIN_PREFLIGHT_MAX_AGE_MS)
      error_code=-1002;
   else
     {
      ResetLastError();
      bool margin_ok=OrderCalcMargin(side=="BUY" ? ORDER_TYPE_BUY : ORDER_TYPE_SELL,
                                     symbol,volume,price,margin);
      error_code=(margin_ok ? 0 : GetLastError());
      if(margin_ok && MathIsValidNumber(margin) && margin>=0.0)
        {
         ResetLastError();
         bool profit_ok=OrderCalcProfit(side=="BUY" ? ORDER_TYPE_BUY : ORDER_TYPE_SELL,
                                        symbol,volume,price,stop_loss,profit_at_stop);
         if(profit_ok && MathIsValidNumber(profit_at_stop) && profit_at_stop<0.0)
            success=true;
         else
           {
            error_code=(profit_ok ? -1003 : GetLastError());
            if(error_code==0) error_code=-1003;
           }
        }
      else if(error_code==0)
         error_code=-1004;
     }

   string margin_json=(success ? Number(margin,8) : "null");
   string loss_json=(success ? Number(MathMax(0.0,-profit_at_stop),8) : "null");
   return StringFormat(
      "{\"requestId\":\"%s\",\"symbol\":\"%s\",\"side\":\"%s\",\"volume\":%s,\"success\":%s,\"calcMargin\":%s,\"lossAtStopLoss\":%s,\"price\":%s,\"bid\":%s,\"ask\":%s,\"quoteTimeMs\":%I64d,\"accountMargin\":%s,\"accountFreeMargin\":%s,\"accountEquity\":%s,\"marginCallLevel\":%s,\"stopOutLevel\":%s,\"marginStopoutMode\":%I64d,\"calculatedAtMs\":%I64d,\"errorCode\":%d}",
      request_id,JsonEscape(symbol),side,Number(volume,8),success ? "true":"false",margin_json,loss_json,
      Number(price,8),tick_ok ? Number(tick.bid,8) : "0",tick_ok ? Number(tick.ask,8) : "0",tick_ok ? tick.time_msc : 0,
      Number(AccountInfoDouble(ACCOUNT_MARGIN),8),Number(AccountInfoDouble(ACCOUNT_MARGIN_FREE),8),
      Number(AccountInfoDouble(ACCOUNT_EQUITY),8),Number(AccountInfoDouble(ACCOUNT_MARGIN_SO_CALL),8),
      Number(AccountInfoDouble(ACCOUNT_MARGIN_SO_SO),8),(long)AccountInfoInteger(ACCOUNT_MARGIN_SO_MODE),
      calculated_at_ms,error_code
   );
  }

void ProcessMarginPreflightAck(const string response)
  {
   string request_object;
   if(!ReadObjectField(response,"marginPreflightRequest",request_object))
      return;
   // Defer calculation until the next heartbeat has captured the account snapshot.
   // The exact MqlTick used by that snapshot is passed into both native calculations.
   if(g_margin_preflight_request_json!="")
     {
      Print("ATOS discarded an overlapping native margin-preflight command.");
      return;
     }
   g_margin_preflight_request_json=request_object;
  }

void CloseBridge(const string reason)
  {
   if(g_socket!=INVALID_HANDLE)
     {
      SocketClose(g_socket);
      g_socket=INVALID_HANDLE;
     }
   g_authenticated=false;
   g_margin_preflight_request_json="";
   g_margin_preflight_results_json="[]";
   if(reason!="")
      Print("ATOS bridge disconnected: ",reason);
  }

string TickSignature(const MqlTick &tick)
  {
   return StringFormat("%I64d|%s|%s|%s|%I64u|%s|%u",
      tick.time_msc,Number(tick.bid),Number(tick.ask),Number(tick.last),
      tick.volume,Number(tick.volume_real),tick.flags);
  }

string TimeframesJson()
  {
   string values[];
   int count=ArraySize(g_timeframes);
   if(ArrayResize(values,count)!=count)
      return "[]";
   for(int i=0;i<count;i++)
      values[i]=TimeframeCode(g_timeframes[i]);
   return StringArrayJson(values);
  }

bool AuthenticatedAccountIdentityMatches()
  {
   return g_account_login!="" && g_account_server!="" && g_account_currency!=""
      && IntegerToString((long)AccountInfoInteger(ACCOUNT_LOGIN))==g_account_login
      && AccountInfoString(ACCOUNT_SERVER)==g_account_server
      && AccountInfoString(ACCOUNT_CURRENCY)==g_account_currency;
  }

bool ConnectBridge()
  {
   if(!AuthenticatedAccountIdentityMatches())
     {
      Print("ATOS bridge refused connection because the active MT5 account identity changed; restart and pair the intended account explicitly.");
      return false;
     }
   if(StringLen(InpPairingToken)!=64)
     {
      Print("ATOS bridge pairing token must contain exactly 64 hexadecimal characters.");
      return false;
     }
   for(int i=0;i<StringLen(InpPairingToken);i++)
     {
      ushort c=StringGetCharacter(InpPairingToken,i);
      if(!((c>='0' && c<='9') || (c>='a' && c<='f') || (c>='A' && c<='F')))
        {
         Print("ATOS bridge pairing token contains a non-hexadecimal character.");
         return false;
        }
     }

   g_socket=SocketCreate(SOCKET_DEFAULT);
   if(g_socket==INVALID_HANDLE)
     {
      Print("ATOS bridge could not create socket. Error ",GetLastError());
      return false;
     }
   SocketTimeouts(g_socket,(uint)InpResponseTimeoutMs,(uint)InpResponseTimeoutMs);
   if(!SocketConnect(g_socket,InpBridgeHost,InpBridgePort,(uint)InpConnectTimeoutMs))
     {
      int error=GetLastError();
      CloseBridge("connection failed, error "+IntegerToString(error));
      return false;
     }

   string nonce=StringFormat("%I64d-%u-%I64d-%I64u",(long)TimeGMT(),GetTickCount(),
      (long)AccountInfoInteger(ACCOUNT_LOGIN),GetMicrosecondCount());
   string auth=StringFormat(
      "{\"protocolVersion\":%d,\"kind\":\"authenticate\",\"token\":\"%s\",\"clientId\":\"%s\",\"sentAtMs\":%I64d,\"nonce\":\"%s\",\"sessionId\":\"%s\",\"accountLogin\":\"%s\",\"brokerServer\":\"%s\",\"symbols\":%s,\"timeframes\":%s}",
      ATOS_PROTOCOL_VERSION,InpPairingToken,JsonEscape(g_client_id),(long)TimeGMT()*1000,
      JsonEscape(nonce),JsonEscape(g_session_id),JsonEscape(g_account_login),
      JsonEscape(AccountInfoString(ACCOUNT_SERVER)),StringArrayJson(g_symbols),TimeframesJson()
   );
   if(!SendLine(auth))
     {
      CloseBridge("authentication frame could not be sent");
      return false;
     }
   string response;
   ulong last_committed=0;
   if(!ReadLine(response,(uint)InpResponseTimeoutMs)
      || StringFind(response,"\"kind\":\"authenticated\"")<0
      || StringFind(response,StringFormat("\"protocolVersion\":%d",ATOS_PROTOCOL_VERSION))<0
      || StringFind(response,StringFormat("\"maxFrameBytes\":%d",ATOS_MAX_FRAME_BYTES))<0
      || !ReadUnsignedField(response,"lastCommittedSequence",last_committed))
     {
      CloseBridge("server did not provide a valid protocol-v6 durable sequence, broker-session, account stop-out, and stop-loss exposure handshake");
      return false;
     }
   for(int symbol_index=0;symbol_index<ArraySize(g_symbols);symbol_index++)
     {
      long tick_cursor=0;
      if(!ReadMapCursor(response,"tickCursors",g_symbols[symbol_index],tick_cursor))
        {
         CloseBridge("server did not return a valid durable tick cursor for "+g_symbols[symbol_index]);
         return false;
        }
      g_last_tick_time[symbol_index]=tick_cursor;
      g_last_tick_signature[symbol_index]="";
      for(int timeframe_index=0;timeframe_index<ArraySize(g_timeframes);timeframe_index++)
        {
         int cursor_index=symbol_index*ArraySize(g_timeframes)+timeframe_index;
         string cursor_key=g_symbols[symbol_index]+"|"+TimeframeCode(g_timeframes[timeframe_index]);
         long bar_cursor=0;
         if(!ReadMapCursor(response,"barCursors",cursor_key,bar_cursor))
           {
            CloseBridge("server did not return a valid durable bar cursor for "+cursor_key);
            return false;
           }
         g_last_bar_time[cursor_index]=bar_cursor;
        }
     }
   g_authenticated=true;
   g_session_calendar_needs_send=true;
   g_sequence=last_committed;
   Print("ATOS MT5 ingestion authenticated for ",ArraySize(g_symbols)," symbols and ",
      ArraySize(g_timeframes)," timeframes. Durable sequence ",g_sequence,".");
   return true;
  }

bool IsBuyOrderType(const ENUM_ORDER_TYPE order_type)
  {
   return order_type==ORDER_TYPE_BUY || order_type==ORDER_TYPE_BUY_LIMIT
      || order_type==ORDER_TYPE_BUY_STOP || order_type==ORDER_TYPE_BUY_STOP_LIMIT;
  }

string StopLossRiskJson(const ENUM_ORDER_TYPE order_type,const string symbol,const double volume,
                        const double reference_price,const double stop_loss,string &risk_status)
  {
   risk_status="UNPROTECTED";
   if(stop_loss<=0.0)
      return "null";
   bool is_buy=IsBuyOrderType(order_type);
   // A stop at, beyond, or on the profitable side of the reference mark is not a
   // protective loss stop. In particular, a position gapped through its SL must not
   // be counted as zero-risk while the terminal is still reporting it as open.
   if(reference_price<=0.0 || !MathIsValidNumber(reference_price)
      || !MathIsValidNumber(stop_loss)
      || (is_buy && stop_loss>=reference_price)
      || (!is_buy && stop_loss<=reference_price))
     {
      risk_status="CALCULATION_FAILED";
      return "null";
     }
   ENUM_ORDER_TYPE market_type=(is_buy ? ORDER_TYPE_BUY : ORDER_TYPE_SELL);
   double estimated_profit=0.0;
   ResetLastError();
   if(!OrderCalcProfit(market_type,symbol,volume,reference_price,stop_loss,estimated_profit)
      || !MathIsValidNumber(estimated_profit))
     {
      risk_status="CALCULATION_FAILED";
      return "null";
     }
   double loss=-estimated_profit;
   if(!MathIsValidNumber(loss) || loss<=0.0)
     {
      risk_status="CALCULATION_FAILED";
      return "null";
     }
   risk_status="CALCULATED";
   return Number(loss,8);
  }

bool SnapshotNumberMatches(const double current_value,const double captured_value)
  {
   if(!MathIsValidNumber(current_value) || !MathIsValidNumber(captured_value))
      return false;
   double scale=MathMax(MathMax(MathAbs(current_value),MathAbs(captured_value)),1.0);
   return MathAbs(current_value-captured_value)<=scale*1.0e-10;
  }

bool BuildSnapshot(string &json,MqlTick &snapshot_tick)
  {
   if(!AuthenticatedAccountIdentityMatches())
     {
      Print("ATOS bridge withheld telemetry because the active MT5 account identity no longer matches the paired identity.");
      return false;
     }
   if(!TerminalInfoInteger(TERMINAL_CONNECTED))
      return false;
   if(!SymbolInfoTick(g_symbol,snapshot_tick) || snapshot_tick.time_msc<=0 || snapshot_tick.bid<=0.0 || snapshot_tick.ask<=0.0 || snapshot_tick.ask<snapshot_tick.bid)
      return false;
   int positions_total=PositionsTotal();
   int orders_total=OrdersTotal();
   if(orders_total>ATOS_MAX_PENDING_ORDERS)
     {
      Print("ATOS bridge refuses an incomplete pending-order snapshot: ",orders_total,
            " active orders exceeds the protocol cap of ",ATOS_MAX_PENDING_ORDERS,".");
      return false;
     }
   if(positions_total>ATOS_MAX_POSITIONS)
     {
      Print("ATOS bridge refuses an incomplete position snapshot: ",positions_total,
            " open positions exceeds the protocol cap of ",ATOS_MAX_POSITIONS,".");
      return false;
     }

   bool account_trade_allowed=(bool)AccountInfoInteger(ACCOUNT_TRADE_ALLOWED);
   bool terminal_trade_allowed=(bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED);
   bool algo_trading_allowed=(bool)MQLInfoInteger(MQL_TRADE_ALLOWED);
   if(ArrayResize(g_snapshot_position_tickets,positions_total)!=positions_total
      || ArrayResize(g_snapshot_position_symbols,positions_total)!=positions_total
      || ArrayResize(g_snapshot_position_types,positions_total)!=positions_total
      || ArrayResize(g_snapshot_position_volumes,positions_total)!=positions_total
      || ArrayResize(g_snapshot_position_prices,positions_total)!=positions_total
      || ArrayResize(g_snapshot_position_stop_losses,positions_total)!=positions_total
      || ArrayResize(g_snapshot_order_tickets,orders_total)!=orders_total
      || ArrayResize(g_snapshot_order_symbols,orders_total)!=orders_total
      || ArrayResize(g_snapshot_order_types,orders_total)!=orders_total
      || ArrayResize(g_snapshot_order_states,orders_total)!=orders_total
      || ArrayResize(g_snapshot_order_initial_volumes,orders_total)!=orders_total
      || ArrayResize(g_snapshot_order_current_volumes,orders_total)!=orders_total
      || ArrayResize(g_snapshot_order_open_prices,orders_total)!=orders_total
      || ArrayResize(g_snapshot_order_stop_limit_prices,orders_total)!=orders_total
      || ArrayResize(g_snapshot_order_stop_losses,orders_total)!=orders_total)
      return false;
   string positions="[";
   int emitted=0;
   for(int i=0;i<positions_total;i++)
     {
      ulong ticket=PositionGetTicket(i);
      if(ticket==0)
         return false;
      if(emitted>0)
         positions+=",";
      string position_symbol=PositionGetString(POSITION_SYMBOL);
      long position_type=PositionGetInteger(POSITION_TYPE);
      double position_volume=PositionGetDouble(POSITION_VOLUME);
      double position_entry=PositionGetDouble(POSITION_PRICE_OPEN);
      double position_current=PositionGetDouble(POSITION_PRICE_CURRENT);
      double position_sl=PositionGetDouble(POSITION_SL);
      g_snapshot_position_tickets[emitted]=ticket;
      g_snapshot_position_symbols[emitted]=position_symbol;
      g_snapshot_position_types[emitted]=position_type;
      g_snapshot_position_volumes[emitted]=position_volume;
      g_snapshot_position_prices[emitted]=position_current;
      g_snapshot_position_stop_losses[emitted]=position_sl;
      string position_risk_status;
      // Equity already includes floating P/L. Measure only the additional move from
      // the current mark to SL so the account-wide stressed-equity projection does
      // not subtract the entry-to-SL loss a second time.
      string position_risk=StopLossRiskJson((ENUM_ORDER_TYPE)position_type,position_symbol,
         position_volume,position_current,position_sl,position_risk_status);
      positions+=StringFormat(
         "{\"ticket\":\"%I64u\",\"symbol\":\"%s\",\"positionType\":%I64d,\"volume\":%s,\"priceOpen\":%s,\"priceCurrent\":%s,\"profit\":%s,\"swap\":%s,\"stopLoss\":%s,\"takeProfit\":%s,\"openedAtMs\":%I64d,\"magic\":\"%I64d\",\"riskStatus\":\"%s\",\"riskAtStopLoss\":%s}",
         ticket,JsonEscape(position_symbol),position_type,
         Number(position_volume),Number(position_entry),
         Number(position_current),Number(PositionGetDouble(POSITION_PROFIT)),
         Number(PositionGetDouble(POSITION_SWAP)),Number(position_sl),
         Number(PositionGetDouble(POSITION_TP)),(long)PositionGetInteger(POSITION_TIME)*1000,
         PositionGetInteger(POSITION_MAGIC),position_risk_status,position_risk
      );
      emitted++;
     }
   positions+="]";

   string pending_orders="[";
   int pending_emitted=0;
   for(int order_index=0;order_index<orders_total;order_index++)
     {
      ulong ticket=OrderGetTicket(order_index);
      if(ticket==0 || !OrderSelect(ticket))
         return false;
      ENUM_ORDER_TYPE order_type=(ENUM_ORDER_TYPE)OrderGetInteger(ORDER_TYPE);
      if(order_type<ORDER_TYPE_BUY_LIMIT || order_type>ORDER_TYPE_SELL_STOP_LIMIT)
         return false;
      double volume_initial=OrderGetDouble(ORDER_VOLUME_INITIAL);
      double volume_current=OrderGetDouble(ORDER_VOLUME_CURRENT);
      double price_open=OrderGetDouble(ORDER_PRICE_OPEN);
      if(volume_initial<=0.0 || volume_current<=0.0 || volume_current>volume_initial || price_open<=0.0)
         return false;
      if(pending_emitted>0)
         pending_orders+=",";
      string pending_symbol=OrderGetString(ORDER_SYMBOL);
      double pending_sl=OrderGetDouble(ORDER_SL);
      double pending_stop_limit=OrderGetDouble(ORDER_PRICE_STOPLIMIT);
      double pending_entry=(order_type==ORDER_TYPE_BUY_STOP_LIMIT || order_type==ORDER_TYPE_SELL_STOP_LIMIT)
         ? pending_stop_limit : price_open;
      g_snapshot_order_tickets[pending_emitted]=ticket;
      g_snapshot_order_symbols[pending_emitted]=pending_symbol;
      g_snapshot_order_types[pending_emitted]=(long)order_type;
      g_snapshot_order_states[pending_emitted]=OrderGetInteger(ORDER_STATE);
      g_snapshot_order_initial_volumes[pending_emitted]=volume_initial;
      g_snapshot_order_current_volumes[pending_emitted]=volume_current;
      g_snapshot_order_open_prices[pending_emitted]=price_open;
      g_snapshot_order_stop_limit_prices[pending_emitted]=pending_stop_limit;
      g_snapshot_order_stop_losses[pending_emitted]=pending_sl;
      string pending_risk_status;
      string pending_risk=StopLossRiskJson(order_type,pending_symbol,volume_current,
         pending_entry,pending_sl,pending_risk_status);
      pending_orders+=StringFormat(
         "{\"ticket\":\"%I64u\",\"symbol\":\"%s\",\"orderType\":%I64d,\"state\":%I64d,\"volumeInitial\":%s,\"volumeCurrent\":%s,\"priceOpen\":%s,\"priceCurrent\":%s,\"priceStopLimit\":%s,\"stopLoss\":%s,\"takeProfit\":%s,\"setupAtMs\":%I64d,\"expirationAtMs\":%I64d,\"magic\":\"%I64d\",\"riskStatus\":\"%s\",\"riskAtStopLoss\":%s}",
         ticket,JsonEscape(pending_symbol),(long)order_type,
         g_snapshot_order_states[pending_emitted],Number(volume_initial),Number(volume_current),
         Number(price_open),Number(OrderGetDouble(ORDER_PRICE_CURRENT)),
         Number(pending_stop_limit),Number(pending_sl),
         Number(OrderGetDouble(ORDER_TP)),(long)OrderGetInteger(ORDER_TIME_SETUP_MSC),
         (long)OrderGetInteger(ORDER_TIME_EXPIRATION)*1000,OrderGetInteger(ORDER_MAGIC),
         pending_risk_status,pending_risk
      );
      pending_emitted++;
     }
   pending_orders+="]";

   // MT5 does not expose an atomic account-inventory snapshot. Fence enumeration by
   // checking complete ticket sets, cardinalities, and every risk-bearing field again.
   // Any concurrent add/remove/resize/reprice/SL change invalidates the frame.
   if(emitted!=positions_total || pending_emitted!=orders_total
      || PositionsTotal()!=positions_total || OrdersTotal()!=orders_total)
     {
      Print("ATOS bridge discarded a non-atomic account inventory snapshot.");
      return false;
     }
   for(int verify_position=0;verify_position<positions_total;verify_position++)
     {
      ulong current_ticket=PositionGetTicket(verify_position);
      if(current_ticket==0)
         return false;
      int captured=-1;
      for(int candidate=0;candidate<positions_total;candidate++)
         if(current_ticket==g_snapshot_position_tickets[candidate])
           { captured=candidate; break; }
      if(captured<0)
        {
         Print("ATOS bridge discarded a position inventory changed during serialization.");
         return false;
        }
      if(PositionGetString(POSITION_SYMBOL)!=g_snapshot_position_symbols[captured]
         || PositionGetInteger(POSITION_TYPE)!=g_snapshot_position_types[captured]
         || !SnapshotNumberMatches(PositionGetDouble(POSITION_VOLUME),g_snapshot_position_volumes[captured])
         || !SnapshotNumberMatches(PositionGetDouble(POSITION_PRICE_CURRENT),g_snapshot_position_prices[captured])
         || !SnapshotNumberMatches(PositionGetDouble(POSITION_SL),g_snapshot_position_stop_losses[captured]))
        {
         Print("ATOS bridge discarded a position whose risk-bearing fields changed during serialization.");
         return false;
        }
     }
   for(int verify_order=0;verify_order<orders_total;verify_order++)
     {
      ulong current_ticket=OrderGetTicket(verify_order);
      if(current_ticket==0 || !OrderSelect(current_ticket))
         return false;
      int captured=-1;
      for(int candidate=0;candidate<orders_total;candidate++)
         if(current_ticket==g_snapshot_order_tickets[candidate])
           { captured=candidate; break; }
      if(captured<0)
        {
         Print("ATOS bridge discarded a pending-order inventory changed during serialization.");
         return false;
        }
      if(OrderGetString(ORDER_SYMBOL)!=g_snapshot_order_symbols[captured]
         || OrderGetInteger(ORDER_TYPE)!=g_snapshot_order_types[captured]
         || OrderGetInteger(ORDER_STATE)!=g_snapshot_order_states[captured]
         || !SnapshotNumberMatches(OrderGetDouble(ORDER_VOLUME_INITIAL),g_snapshot_order_initial_volumes[captured])
         || !SnapshotNumberMatches(OrderGetDouble(ORDER_VOLUME_CURRENT),g_snapshot_order_current_volumes[captured])
         || !SnapshotNumberMatches(OrderGetDouble(ORDER_PRICE_OPEN),g_snapshot_order_open_prices[captured])
         || !SnapshotNumberMatches(OrderGetDouble(ORDER_PRICE_STOPLIMIT),g_snapshot_order_stop_limit_prices[captured])
         || !SnapshotNumberMatches(OrderGetDouble(ORDER_SL),g_snapshot_order_stop_losses[captured]))
        {
         Print("ATOS bridge discarded a pending order whose risk-bearing fields changed during serialization.");
         return false;
        }
     }

   long server_time=(long)TimeTradeServer()*1000;
   double account_balance=AccountInfoDouble(ACCOUNT_BALANCE);
   double account_equity=AccountInfoDouble(ACCOUNT_EQUITY);
   double account_credit=AccountInfoDouble(ACCOUNT_CREDIT);
   double account_profit=AccountInfoDouble(ACCOUNT_PROFIT);
   double account_margin=AccountInfoDouble(ACCOUNT_MARGIN);
   double account_free_margin=AccountInfoDouble(ACCOUNT_MARGIN_FREE);
   double account_margin_level=AccountInfoDouble(ACCOUNT_MARGIN_LEVEL);
   double account_margin_initial=AccountInfoDouble(ACCOUNT_MARGIN_INITIAL);
   double account_margin_maintenance=AccountInfoDouble(ACCOUNT_MARGIN_MAINTENANCE);
   double account_margin_call_level=AccountInfoDouble(ACCOUNT_MARGIN_SO_CALL);
   double account_stop_out_level=AccountInfoDouble(ACCOUNT_MARGIN_SO_SO);
   long account_margin_stopout_mode=(long)AccountInfoInteger(ACCOUNT_MARGIN_SO_MODE);
   long account_margin_mode=(long)AccountInfoInteger(ACCOUNT_MARGIN_MODE);
   long account_limit_orders=(long)AccountInfoInteger(ACCOUNT_LIMIT_ORDERS);
   long account_leverage=(long)AccountInfoInteger(ACCOUNT_LEVERAGE);
   bool account_hedge_allowed=(bool)AccountInfoInteger(ACCOUNT_HEDGE_ALLOWED);
   g_snapshot_account_balance=account_balance;
   g_snapshot_account_equity=account_equity;
   g_snapshot_account_credit=account_credit;
   g_snapshot_account_profit=account_profit;
   g_snapshot_account_margin=account_margin;
   g_snapshot_account_free_margin=account_free_margin;
   g_snapshot_account_margin_level=account_margin_level;
   g_snapshot_account_margin_initial=account_margin_initial;
   g_snapshot_account_margin_maintenance=account_margin_maintenance;
   g_snapshot_account_margin_call_level=account_margin_call_level;
   g_snapshot_account_stop_out_level=account_stop_out_level;
   g_snapshot_account_margin_stopout_mode=account_margin_stopout_mode;
   g_snapshot_account_margin_mode=account_margin_mode;
   g_snapshot_account_limit_orders=account_limit_orders;
   g_snapshot_account_leverage=account_leverage;
   g_snapshot_account_hedge_allowed=(bool)AccountInfoInteger(ACCOUNT_HEDGE_ALLOWED);
   g_snapshot_account_trade_allowed=account_trade_allowed;
   g_snapshot_terminal_trade_allowed=terminal_trade_allowed;
   g_snapshot_algo_trading_allowed=algo_trading_allowed;
   string format=
      "{\"account\":{\"login\":\"%s\",\"server\":\"%s\",\"currency\":\"%s\",\"balance\":%s,\"equity\":%s,\"credit\":%s,\"profit\":%s,\"margin\":%s,\"freeMargin\":%s,\"marginLevelPct\":%s,\"marginInitial\":%s,\"marginMaintenance\":%s,\"marginCallLevel\":%s,\"stopOutLevel\":%s,\"marginStopoutMode\":%I64d,\"marginMode\":%I64d,\"accountHedgeAllowed\":%s,\"accountLimitOrders\":%I64d,\"leverage\":%I64d,\"tradeAllowed\":%s,\"accountTradeAllowed\":%s,\"terminalTradeAllowed\":%s},"+
      "\"terminal\":{\"name\":\"%s\",\"company\":\"%s\",\"build\":%I64d,\"connected\":true,\"algoTradingAllowed\":%s,\"serverTimeMs\":%I64d},"+
      "\"symbol\":{\"name\":\"%s\",\"bid\":%s,\"ask\":%s,\"point\":%s,\"tickSize\":%s,\"tickValue\":%s,\"volumeMin\":%s,\"volumeMax\":%s,\"volumeStep\":%s,\"digits\":%I64d,\"tradeMode\":%I64d,\"quoteTimeMs\":%I64d,\"contractSize\":%s,\"calculationMode\":%I64d,\"executionMode\":%I64d,\"fillingMode\":%I64d,\"orderMode\":%I64d,\"tickValueProfit\":%s,\"tickValueLoss\":%s,\"volumeLimit\":%s,\"stopsLevelPoints\":%I64d,\"freezeLevelPoints\":%I64d,\"currencyBase\":\"%s\",\"currencyProfit\":\"%s\",\"currencyMargin\":\"%s\",\"swapMode\":%I64d,\"swapLong\":%s,\"swapShort\":%s},"+
      "\"positions\":%s,\"pendingOrders\":%s}";
   json=StringFormat(format,
      g_account_login,JsonEscape(AccountInfoString(ACCOUNT_SERVER)),JsonEscape(AccountInfoString(ACCOUNT_CURRENCY)),
      Number(account_balance,8),Number(account_equity,8),
      Number(account_credit,8),Number(account_profit,8),
      Number(account_margin,8),Number(account_free_margin,8),
      Number(account_margin_level,8),Number(account_margin_initial,8),
      Number(account_margin_maintenance,8),Number(account_margin_call_level,8),
      Number(account_stop_out_level,8),account_margin_stopout_mode,
      account_margin_mode,
      account_hedge_allowed ? "true":"false",
      account_limit_orders,account_leverage,
      account_trade_allowed && terminal_trade_allowed && algo_trading_allowed ? "true":"false",
      account_trade_allowed ? "true":"false",terminal_trade_allowed ? "true":"false",
      JsonEscape(TerminalInfoString(TERMINAL_NAME)),JsonEscape(TerminalInfoString(TERMINAL_COMPANY)),
      (long)TerminalInfoInteger(TERMINAL_BUILD),algo_trading_allowed ? "true":"false",server_time,
      JsonEscape(g_symbol),Number(snapshot_tick.bid),Number(snapshot_tick.ask),Number(SymbolInfoDouble(g_symbol,SYMBOL_POINT)),
      Number(SymbolInfoDouble(g_symbol,SYMBOL_TRADE_TICK_SIZE)),Number(SymbolInfoDouble(g_symbol,SYMBOL_TRADE_TICK_VALUE)),
      Number(SymbolInfoDouble(g_symbol,SYMBOL_VOLUME_MIN)),Number(SymbolInfoDouble(g_symbol,SYMBOL_VOLUME_MAX)),
      Number(SymbolInfoDouble(g_symbol,SYMBOL_VOLUME_STEP)),(long)SymbolInfoInteger(g_symbol,SYMBOL_DIGITS),
      (long)SymbolInfoInteger(g_symbol,SYMBOL_TRADE_MODE),snapshot_tick.time_msc,
      Number(SymbolInfoDouble(g_symbol,SYMBOL_TRADE_CONTRACT_SIZE)),
      (long)SymbolInfoInteger(g_symbol,SYMBOL_TRADE_CALC_MODE),
      (long)SymbolInfoInteger(g_symbol,SYMBOL_TRADE_EXEMODE),
      (long)SymbolInfoInteger(g_symbol,SYMBOL_FILLING_MODE),
      (long)SymbolInfoInteger(g_symbol,SYMBOL_ORDER_MODE),
      Number(SymbolInfoDouble(g_symbol,SYMBOL_TRADE_TICK_VALUE_PROFIT)),
      Number(SymbolInfoDouble(g_symbol,SYMBOL_TRADE_TICK_VALUE_LOSS)),
      Number(SymbolInfoDouble(g_symbol,SYMBOL_VOLUME_LIMIT)),
      (long)SymbolInfoInteger(g_symbol,SYMBOL_TRADE_STOPS_LEVEL),
      (long)SymbolInfoInteger(g_symbol,SYMBOL_TRADE_FREEZE_LEVEL),
      JsonEscape(SymbolInfoString(g_symbol,SYMBOL_CURRENCY_BASE)),
      JsonEscape(SymbolInfoString(g_symbol,SYMBOL_CURRENCY_PROFIT)),
      JsonEscape(SymbolInfoString(g_symbol,SYMBOL_CURRENCY_MARGIN)),
      (long)SymbolInfoInteger(g_symbol,SYMBOL_SWAP_MODE),
      Number(SymbolInfoDouble(g_symbol,SYMBOL_SWAP_LONG)),
      Number(SymbolInfoDouble(g_symbol,SYMBOL_SWAP_SHORT)),positions,pending_orders
   );
   return StringLen(json)>0;
  }

bool AccountRiskFieldsMatchSnapshot()
  {
   return AuthenticatedAccountIdentityMatches()
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_BALANCE),g_snapshot_account_balance)
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_EQUITY),g_snapshot_account_equity)
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_CREDIT),g_snapshot_account_credit)
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_PROFIT),g_snapshot_account_profit)
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_MARGIN),g_snapshot_account_margin)
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_MARGIN_FREE),g_snapshot_account_free_margin)
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_MARGIN_LEVEL),g_snapshot_account_margin_level)
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_MARGIN_INITIAL),g_snapshot_account_margin_initial)
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_MARGIN_MAINTENANCE),g_snapshot_account_margin_maintenance)
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_MARGIN_SO_CALL),g_snapshot_account_margin_call_level)
      && SnapshotNumberMatches(AccountInfoDouble(ACCOUNT_MARGIN_SO_SO),g_snapshot_account_stop_out_level)
      && (long)AccountInfoInteger(ACCOUNT_MARGIN_SO_MODE)==g_snapshot_account_margin_stopout_mode
      && (long)AccountInfoInteger(ACCOUNT_MARGIN_MODE)==g_snapshot_account_margin_mode
      && (long)AccountInfoInteger(ACCOUNT_LIMIT_ORDERS)==g_snapshot_account_limit_orders
      && (long)AccountInfoInteger(ACCOUNT_LEVERAGE)==g_snapshot_account_leverage
      && (bool)AccountInfoInteger(ACCOUNT_HEDGE_ALLOWED)==g_snapshot_account_hedge_allowed
      && (bool)AccountInfoInteger(ACCOUNT_TRADE_ALLOWED)==g_snapshot_account_trade_allowed
      && (bool)TerminalInfoInteger(TERMINAL_TRADE_ALLOWED)==g_snapshot_terminal_trade_allowed
      && (bool)MQLInfoInteger(MQL_TRADE_ALLOWED)==g_snapshot_algo_trading_allowed;
  }

bool AccountInventoryMatchesLastSnapshot()
  {
   if(!AccountRiskFieldsMatchSnapshot())
      return false;
   int positions_total=ArraySize(g_snapshot_position_tickets);
   int orders_total=ArraySize(g_snapshot_order_tickets);
   if(PositionsTotal()!=positions_total || OrdersTotal()!=orders_total)
      return false;
   for(int index=0;index<positions_total;index++)
     {
      ulong ticket=PositionGetTicket(index);
      if(ticket==0)
         return false;
      int captured=-1;
      for(int candidate=0;candidate<positions_total;candidate++)
         if(g_snapshot_position_tickets[candidate]==ticket)
           { captured=candidate; break; }
      if(captured<0)
         return false;
      string symbol=PositionGetString(POSITION_SYMBOL);
      long position_type=PositionGetInteger(POSITION_TYPE);
      double volume=PositionGetDouble(POSITION_VOLUME);
      double current_price=PositionGetDouble(POSITION_PRICE_CURRENT);
      double stop_loss=PositionGetDouble(POSITION_SL);
      if(symbol!=g_snapshot_position_symbols[captured]
         || position_type!=g_snapshot_position_types[captured]
         || !SnapshotNumberMatches(volume,g_snapshot_position_volumes[captured])
         || !SnapshotNumberMatches(current_price,g_snapshot_position_prices[captured])
         || !SnapshotNumberMatches(stop_loss,g_snapshot_position_stop_losses[captured]))
         return false;
     }
   for(int index=0;index<orders_total;index++)
     {
      ulong ticket=OrderGetTicket(index);
      if(ticket==0 || !OrderSelect(ticket))
         return false;
      int captured=-1;
      for(int candidate=0;candidate<orders_total;candidate++)
         if(g_snapshot_order_tickets[candidate]==ticket)
           { captured=candidate; break; }
      if(captured<0)
         return false;
      string symbol=OrderGetString(ORDER_SYMBOL);
      long order_type=OrderGetInteger(ORDER_TYPE);
      long state=OrderGetInteger(ORDER_STATE);
      double volume_initial=OrderGetDouble(ORDER_VOLUME_INITIAL);
      double volume_current=OrderGetDouble(ORDER_VOLUME_CURRENT);
      double price_open=OrderGetDouble(ORDER_PRICE_OPEN);
      double price_stop_limit=OrderGetDouble(ORDER_PRICE_STOPLIMIT);
      double stop_loss=OrderGetDouble(ORDER_SL);
      if(symbol!=g_snapshot_order_symbols[captured]
         || order_type!=g_snapshot_order_types[captured]
         || state!=g_snapshot_order_states[captured]
         || !SnapshotNumberMatches(volume_initial,g_snapshot_order_initial_volumes[captured])
         || !SnapshotNumberMatches(volume_current,g_snapshot_order_current_volumes[captured])
         || !SnapshotNumberMatches(price_open,g_snapshot_order_open_prices[captured])
         || !SnapshotNumberMatches(price_stop_limit,g_snapshot_order_stop_limit_prices[captured])
         || !SnapshotNumberMatches(stop_loss,g_snapshot_order_stop_losses[captured]))
         return false;
     }
   return true;
  }

bool BuildMarketData(string &json,long &candidate_tick_times[],string &candidate_tick_signatures[],long &candidate_bar_times[],long &candidate_bar_audit_server_time)
  {
   ArrayCopy(candidate_tick_times,g_last_tick_time);
   ArrayCopy(candidate_tick_signatures,g_last_tick_signature);
   ArrayCopy(candidate_bar_times,g_last_bar_time);
   string ticks_json="[";
   string bars_json="[";
   int tick_count=0;
   int bar_count=0;
   long server_now=(long)TimeTradeServer();
   if(server_now<=0)
      return false;
   bool audit_recent_bars=(g_last_bar_audit_server_time<=0
      || server_now<g_last_bar_audit_server_time
      || server_now-g_last_bar_audit_server_time>=ATOS_BAR_AUDIT_INTERVAL_SECONDS);
   candidate_bar_audit_server_time=audit_recent_bars ? server_now : g_last_bar_audit_server_time;

   for(int symbol_index=0;symbol_index<ArraySize(g_symbols);symbol_index++)
     {
      string watched=g_symbols[symbol_index];
      // CopyTicks(from=0,count=N) is documented to return the most recent N ticks.
      // A zero durable cursor therefore bootstraps a bounded live-edge window, not the
      // entire history. Subsequent sessions resume from the SQLite cursor.
      long effective_tick_cursor=g_last_tick_time[symbol_index];
      MqlTick ticks[];
      ResetLastError();
      int copied_ticks=CopyTicks(watched,ticks,COPY_TICKS_ALL,(ulong)MathMax(0,effective_tick_cursor),ATOS_TICKS_PER_SYMBOL);
      if(copied_ticks<0)
        {
         Print("ATOS withheld frame: CopyTicks failed for ",watched,". Error ",GetLastError());
         return false;
        }
      bool cursor_seen=(effective_tick_cursor<=0 || g_last_tick_signature[symbol_index]=="");
      bool cursor_boundary_found=cursor_seen;
      for(int tick_index=0;tick_index<copied_ticks;tick_index++)
        {
         MqlTick current=ticks[tick_index];
         if(current.time_msc<=0)
            return false;
         string signature=TickSignature(current);
         candidate_tick_times[symbol_index]=current.time_msc;
         candidate_tick_signatures[symbol_index]=signature;
         if(!cursor_seen)
           {
            if(current.time_msc<effective_tick_cursor)
               continue;
            if(current.time_msc==effective_tick_cursor && signature==g_last_tick_signature[symbol_index])
              {
               cursor_seen=true;
               cursor_boundary_found=true;
               continue;
              }
            if(current.time_msc>effective_tick_cursor)
               cursor_seen=true;
           }
         if(tick_count>=ATOS_MAX_SYMBOLS*ATOS_TICKS_PER_SYMBOL)
            return false;
         if(tick_count>0)
            ticks_json+=",";
         ticks_json+=StringFormat(
            "{\"symbol\":\"%s\",\"timeMs\":%I64d,\"bid\":%s,\"ask\":%s,\"last\":%s,\"volume\":%I64u,\"volumeReal\":%s,\"flags\":%u}",
            JsonEscape(watched),current.time_msc,Number(current.bid),Number(current.ask),Number(current.last),
            current.volume,Number(current.volume_real),current.flags);
         tick_count++;
        }
      if(!cursor_boundary_found && copied_ticks>0)
        {
         Print("ATOS withheld frame: the last committed tick boundary is missing from terminal history for ",
               watched,". Refusing to skip potentially uncommitted same-millisecond ticks.");
         return false;
        }
      MqlTick latest_after_copy;
      if(!SymbolInfoTick(watched,latest_after_copy) || latest_after_copy.time_msc<=0)
         return false;
      if(effective_tick_cursor>0 && latest_after_copy.time_msc<effective_tick_cursor)
        {
         Print("ATOS withheld frame: the terminal tick clock moved behind the durable cursor for ",watched,
               ". Check broker history, account server, and terminal time consistency.");
         return false;
        }
      if(copied_ticks<ATOS_TICKS_PER_SYMBOL && candidate_tick_times[symbol_index]<latest_after_copy.time_msc)
        {
         Print("ATOS withheld frame: tick history gap detected for ",watched,
               "; durable cursor is behind the current terminal tick. Check MT5 tick-history availability.");
         return false;
        }

      for(int timeframe_index=0;timeframe_index<ArraySize(g_timeframes);timeframe_index++)
        {
         int cursor_index=symbol_index*ArraySize(g_timeframes)+timeframe_index;
         MqlRates rates[];
         MqlRates latest_closed_probe[];
         ResetLastError();
         int latest_probe=CopyRates(watched,g_timeframes[timeframe_index],1,1,latest_closed_probe);
         if(latest_probe!=1)
           {
            Print("ATOS withheld frame: terminal could not synchronize the latest closed bar for ",
                  watched," ",TimeframeCode(g_timeframes[timeframe_index]),". Error ",GetLastError());
            return false;
           }
         int start_position=1;
         int requested_bars=ATOS_CLOSED_BARS_PER_SERIES;
         long durable_bar_cursor=candidate_bar_times[cursor_index];
         if(durable_bar_cursor>0)
           {
            MqlRates cursor_probe[];
            int probe_count=CopyRates(watched,g_timeframes[timeframe_index],(datetime)(durable_bar_cursor/1000),1,cursor_probe);
            if(probe_count!=1 || (long)cursor_probe[0].time*1000!=durable_bar_cursor)
              {
               Print("ATOS withheld frame: exact durable bar timestamp is not available from broker history for ",
                     watched," ",TimeframeCode(g_timeframes[timeframe_index]),". History synchronization failed.");
               return false;
              }
            int cursor_shift=iBarShift(watched,g_timeframes[timeframe_index],(datetime)(durable_bar_cursor/1000),true);
            if(cursor_shift<1)
              {
               Print("ATOS withheld frame: durable bar cursor is unavailable in terminal history for ",
                     watched," ",TimeframeCode(g_timeframes[timeframe_index]),". History reset or missing history requires operator review.");
               return false;
              }
            if(cursor_shift==1)
               requested_bars=0;
            else
              {
               requested_bars=(int)MathMin((double)ATOS_CLOSED_BARS_PER_SERIES,(double)(cursor_shift-1));
               start_position=cursor_shift-requested_bars;
              }
           }
         if(durable_bar_cursor>0)
           {
            if(requested_bars>0)
              {
               ResetLastError();
               int copied_bars=CopyRates(watched,g_timeframes[timeframe_index],start_position,requested_bars,rates);
               if(copied_bars!=requested_bars)
                 {
                  Print("ATOS withheld frame: closed-bar recovery chunk is incomplete for ",watched," ",
                        TimeframeCode(g_timeframes[timeframe_index]),". Requested ",requested_bars,
                        ", copied ",copied_bars,". Error ",GetLastError());
                  return false;
                 }
              }
            else
               ArrayResize(rates,0);
           }
         else
           {
            ResetLastError();
            int copied_bars=CopyRates(watched,g_timeframes[timeframe_index],1,ATOS_CLOSED_BARS_PER_SERIES,rates);
            if(copied_bars<0)
              {
               Print("ATOS withheld frame: initial closed-bar history read failed for ",watched," ",
                     TimeframeCode(g_timeframes[timeframe_index]),". Error ",GetLastError());
               return false;
              }
           }
         for(int bar_index=0;bar_index<ArraySize(rates);bar_index++)
           {
            MqlRates rate=rates[bar_index];
            long bar_time_ms=(long)rate.time*1000;
            if(bar_time_ms<=candidate_bar_times[cursor_index])
               continue;
            if(bar_count>=ATOS_MAX_SYMBOLS*ATOS_MAX_TIMEFRAMES*(ATOS_CLOSED_BARS_PER_SERIES+ATOS_BAR_AUDIT_LOOKBACK))
               return false;
            if(bar_count>0)
               bars_json+=",";
            bars_json+=StringFormat(
               "{\"symbol\":\"%s\",\"timeframe\":\"%s\",\"timeMs\":%I64d,\"open\":%s,\"high\":%s,\"low\":%s,\"close\":%s,\"tickVolume\":%I64d,\"spread\":%d,\"realVolume\":%I64d,\"closed\":true}",
               JsonEscape(watched),TimeframeCode(g_timeframes[timeframe_index]),bar_time_ms,
               Number(rate.open),Number(rate.high),Number(rate.low),Number(rate.close),
               rate.tick_volume,rate.spread,rate.real_volume);
            candidate_bar_times[cursor_index]=bar_time_ms;
            bar_count++;
           }
         if(audit_recent_bars && durable_bar_cursor>0)
           {
            MqlRates audit_rates[];
            ResetLastError();
            int audit_count=CopyRates(watched,g_timeframes[timeframe_index],1,ATOS_BAR_AUDIT_LOOKBACK,audit_rates);
            if(audit_count<0)
              {
               Print("ATOS withheld frame: recent closed-bar audit failed for ",watched," ",
                     TimeframeCode(g_timeframes[timeframe_index]),". Error ",GetLastError());
               return false;
              }
            for(int audit_index=0;audit_index<audit_count;audit_index++)
              {
               MqlRates audited=audit_rates[audit_index];
               long audited_time_ms=(long)audited.time*1000;
               if(audited_time_ms<=0 || audited_time_ms>durable_bar_cursor)
                  continue;
               if(bar_count>=ATOS_MAX_SYMBOLS*ATOS_MAX_TIMEFRAMES*(ATOS_CLOSED_BARS_PER_SERIES+ATOS_BAR_AUDIT_LOOKBACK))
                  return false;
               if(bar_count>0)
                  bars_json+=",";
               bars_json+=StringFormat(
                  "{\"symbol\":\"%s\",\"timeframe\":\"%s\",\"timeMs\":%I64d,\"open\":%s,\"high\":%s,\"low\":%s,\"close\":%s,\"tickVolume\":%I64d,\"spread\":%d,\"realVolume\":%I64d,\"closed\":true}",
                  JsonEscape(watched),TimeframeCode(g_timeframes[timeframe_index]),audited_time_ms,
                  Number(audited.open),Number(audited.high),Number(audited.low),Number(audited.close),
                  audited.tick_volume,audited.spread,audited.real_volume);
               bar_count++;
              }
           }
        }
     }
   ticks_json+="]";
   bars_json+="]";
   json="{\"ticks\":"+ticks_json+",\"bars\":"+bars_json+"}";
   return StringLen(json)>0;
  }

// Capture the complete seven-day broker-server trading schedule for every
// authenticated symbol. Session times are seconds from midnight in broker
// server wall-clock space; they are not UTC and are never offset-converted here.
bool BuildSessionSchedules(string &json,bool &include_schedule)
  {
   include_schedule=false;
   uint now=GetTickCount();
   uint refresh_interval=(uint)InpSessionCalendarRefreshSeconds*1000;
   if(StringLen(g_session_schedules_cache)>2
      && (uint)(now-g_last_session_calendar_refresh)<refresh_interval)
     {
      if(g_session_calendar_needs_send)
        {
         json=g_session_schedules_cache;
         include_schedule=true;
        }
      else
         json="[]";
      return true;
     }
   json="[";
   for(int symbol_index=0;symbol_index<ArraySize(g_symbols);symbol_index++)
     {
      string watched=g_symbols[symbol_index];
      if(symbol_index>0) json+=",";
      json+=StringFormat("{\"symbol\":\"%s\",\"capturedAtMs\":%I64d,\"days\":[",
                         JsonEscape(watched),(long)TimeGMT()*1000);
      for(int day=0;day<7;day++)
        {
         if(day>0) json+=",";
         json+=StringFormat("{\"dayOfWeek\":%d,\"sessions\":[",day);
         bool first_session=true;
         int session_index=0;
         for(;session_index<16;session_index++)
           {
            datetime session_from=0;
            datetime session_to=0;
            ResetLastError();
            if(!SymbolInfoSessionTrade(watched,(ENUM_DAY_OF_WEEK)day,(uint)session_index,session_from,session_to))
              {
               int session_error=GetLastError();
               if(session_error!=0 && session_error!=4307)
                 {
                  Print("ATOS withheld frame: unable to read broker trading sessions for ",watched,
                        " weekday ",day,". Error ",session_error);
                  return false;
                 }
               break;
              }
            long from_seconds=(long)session_from;
            long to_seconds=(long)session_to;
            if(from_seconds<0 || from_seconds>86400 || to_seconds<0 || to_seconds>86400)
              {
               Print("ATOS withheld frame: broker returned invalid trading-session boundaries for ",watched,
                     " weekday ",day," session ",session_index);
               return false;
              }
            if(!first_session) json+=",";
            json+=StringFormat("{\"index\":%d,\"fromSeconds\":%I64u,\"toSeconds\":%I64u}",
                               session_index,(ulong)from_seconds,(ulong)to_seconds);
            first_session=false;
           }
         if(session_index==16)
           {
            datetime overflow_from=0;
            datetime overflow_to=0;
            ResetLastError();
            if(SymbolInfoSessionTrade(watched,(ENUM_DAY_OF_WEEK)day,16,overflow_from,overflow_to))
              {
               Print("ATOS withheld frame: broker session count exceeds the supported per-day bound for ",watched,
                     " weekday ",day);
               return false;
              }
            int overflow_error=GetLastError();
            if(overflow_error!=0 && overflow_error!=4307)
               return false;
           }
         json+="]}";
        }
      json+="]}";
     }
   json+="]";
   if(StringLen(json)<=2)
      return false;
   g_session_schedules_cache=json;
   g_last_session_calendar_refresh=GetTickCount();
   include_schedule=true;
   return true;
  }

void SendHeartbeat()
  {
   if(!g_authenticated || g_socket==INVALID_HANDLE || !SocketIsConnected(g_socket))
     {
      CloseBridge("socket is no longer connected");
      return;
     }
   string snapshot;
   MqlTick snapshot_tick={};
   if(!BuildSnapshot(snapshot,snapshot_tick))
     {
      Print("ATOS withheld frame because terminal, quote, or position snapshot was incomplete.");
      return;
     }
   if(g_margin_preflight_request_json!="")
     {
      string native_result=BuildMarginPreflightResult(g_margin_preflight_request_json,snapshot_tick);
      g_margin_preflight_request_json="";
      if(native_result!="")
        {
         g_margin_preflight_results_json="["+native_result+"]";
        }
      else
         Print("ATOS rejected a malformed or out-of-scope native margin-preflight command.");
     }
   long candidate_tick_times[];
   string candidate_tick_signatures[];
   long candidate_bar_times[];
   long candidate_bar_audit_server_time=0;
   string market_data;
   if(!BuildMarketData(market_data,candidate_tick_times,candidate_tick_signatures,candidate_bar_times,candidate_bar_audit_server_time))
      return;
   string session_schedules;
   bool include_session_schedule=false;
   if(!BuildSessionSchedules(session_schedules,include_session_schedule))
      return;
   if(!AccountInventoryMatchesLastSnapshot())
     {
      g_margin_preflight_results_json="[]";
      Print("ATOS discarded telemetry because account financial state or inventory changed before transmission.");
      return;
     }
   ulong next_sequence=g_sequence+1;
   string frame=StringFormat(
      "{\"protocolVersion\":%d,\"kind\":\"snapshot\",\"sequence\":%I64u,\"sentAtMs\":%I64d,\"snapshot\":%s,\"marketData\":%s,\"sessionSchedules\":%s,\"marginPreflightResults\":%s}",
      ATOS_PROTOCOL_VERSION,next_sequence,(long)TimeGMT()*1000,snapshot,market_data,session_schedules,g_margin_preflight_results_json);
   if(!SendLine(frame))
     {
      CloseBridge("telemetry frame could not be sent completely");
      return;
     }
   string response;
   ulong acknowledged_sequence=0;
   if(!ReadLine(response,(uint)InpResponseTimeoutMs)
      || StringFind(response,"\"kind\":\"ack\"")<0
      || !ReadUnsignedField(response,"sequence",acknowledged_sequence)
      || acknowledged_sequence!=next_sequence
      || StringFind(response,"\"accepted\":true")<0)
     {
      CloseBridge("server did not acknowledge the exact durable telemetry sequence");
      return;
     }
   g_sequence=next_sequence;
   g_margin_preflight_results_json="[]";
   ProcessMarginPreflightAck(response);
   if(include_session_schedule)
      g_session_calendar_needs_send=false;
   ArrayCopy(g_last_tick_time,candidate_tick_times);
   ArrayCopy(g_last_tick_signature,candidate_tick_signatures);
   ArrayCopy(g_last_bar_time,candidate_bar_times);
   g_last_bar_audit_server_time=candidate_bar_audit_server_time;
  }

string TimeframeCode(const ENUM_TIMEFRAMES timeframe)
  {
   switch(timeframe)
     {
      case PERIOD_M1: return "M1";
      case PERIOD_M2: return "M2";
      case PERIOD_M3: return "M3";
      case PERIOD_M4: return "M4";
      case PERIOD_M5: return "M5";
      case PERIOD_M6: return "M6";
      case PERIOD_M10: return "M10";
      case PERIOD_M12: return "M12";
      case PERIOD_M15: return "M15";
      case PERIOD_M20: return "M20";
      case PERIOD_M30: return "M30";
      case PERIOD_H1: return "H1";
      case PERIOD_H2: return "H2";
      case PERIOD_H3: return "H3";
      case PERIOD_H4: return "H4";
      case PERIOD_H6: return "H6";
      case PERIOD_H8: return "H8";
      case PERIOD_H12: return "H12";
      case PERIOD_D1: return "D1";
      case PERIOD_W1: return "W1";
      case PERIOD_MN1: return "MN1";
     }
   return "";
  }

ENUM_TIMEFRAMES ParseTimeframe(string raw)
  {
   string value=raw;
   StringTrimLeft(value);
   StringTrimRight(value);
   StringToUpper(value);
   if(value=="M1") return PERIOD_M1;
   if(value=="M2") return PERIOD_M2;
   if(value=="M3") return PERIOD_M3;
   if(value=="M4") return PERIOD_M4;
   if(value=="M5") return PERIOD_M5;
   if(value=="M6") return PERIOD_M6;
   if(value=="M10") return PERIOD_M10;
   if(value=="M12") return PERIOD_M12;
   if(value=="M15") return PERIOD_M15;
   if(value=="M20") return PERIOD_M20;
   if(value=="M30") return PERIOD_M30;
   if(value=="H1") return PERIOD_H1;
   if(value=="H2") return PERIOD_H2;
   if(value=="H3") return PERIOD_H3;
   if(value=="H4") return PERIOD_H4;
   if(value=="H6") return PERIOD_H6;
   if(value=="H8") return PERIOD_H8;
   if(value=="H12") return PERIOD_H12;
   if(value=="D1") return PERIOD_D1;
   if(value=="W1") return PERIOD_W1;
   if(value=="MN1") return PERIOD_MN1;
   return PERIOD_CURRENT;
  }

bool IsSafeSymbolName(const string value)
  {
   int length=StringLen(value);
   if(length<=0 || length>64 || StringGetCharacter(value,0)==' ' || StringGetCharacter(value,length-1)==' ')
      return false;
   for(int i=0;i<length;i++)
     {
      ushort character=StringGetCharacter(value,i);
      if(character<32 || character>126 || character==34 || character==92 || character==124 || character==44)
         return false;
     }
   return true;
  }

bool AddWatchedSymbol(const string raw)
  {
   string value=raw;
   StringTrimLeft(value);
   StringTrimRight(value);
   if(StringLen(value)==0)
      return true;
   if(!IsSafeSymbolName(value))
     {
      Print("ATOS rejected a broker symbol containing unsupported control, JSON, or cursor-delimiter characters: ",value);
      return false;
     }
   if(ArraySize(g_symbols)>=ATOS_MAX_SYMBOLS)
      return false;
   for(int i=0;i<ArraySize(g_symbols);i++)
      if(g_symbols[i]==value)
         return true;
   if(!SymbolSelect(value,true))
     {
      Print("ATOS could not select broker symbol ",value,". Error ",GetLastError());
      return false;
     }
   int size=ArraySize(g_symbols);
   if(ArrayResize(g_symbols,size+1)!=size+1)
      return false;
   g_symbols[size]=value;
   return true;
  }

int OnInit()
  {
   if(InpBridgeHost!="127.0.0.1")
     {
      Print("ATOS bridge refuses non-loopback hosts. Remote MT5 connectivity is not supported by this transport.");
      return INIT_PARAMETERS_INCORRECT;
     }
   if(InpBridgePort<1 || InpBridgePort>65535 || InpHeartbeatSeconds<1 || InpHeartbeatSeconds>10
      || InpConnectTimeoutMs<250 || InpConnectTimeoutMs>5000 || InpResponseTimeoutMs<500 || InpResponseTimeoutMs>10000
      || InpSessionCalendarRefreshSeconds<30 || InpSessionCalendarRefreshSeconds>3600)
      return INIT_PARAMETERS_INCORRECT;

   g_symbol=InpWatchedSymbol=="" ? _Symbol : InpWatchedSymbol;
   if(!AddWatchedSymbol(g_symbol))
      return INIT_FAILED;
   if(StringLen(InpWatchSymbols)>0)
     {
      string configured_symbols[];
      ushort comma=StringGetCharacter(",",0);
      int symbol_count=StringSplit(InpWatchSymbols,comma,configured_symbols);
      for(int i=0;i<symbol_count;i++)
         if(!AddWatchedSymbol(configured_symbols[i]))
            return INIT_PARAMETERS_INCORRECT;
     }

   string configured_timeframes[];
   ushort timeframe_comma=StringGetCharacter(",",0);
   int timeframe_count=StringSplit(InpTimeframes,timeframe_comma,configured_timeframes);
   if(timeframe_count<=0 || timeframe_count>ATOS_MAX_TIMEFRAMES)
     {
      Print("ATOS timeframe count must be between 1 and ",ATOS_MAX_TIMEFRAMES,".");
      return INIT_PARAMETERS_INCORRECT;
     }
   for(int i=0;i<timeframe_count;i++)
     {
      ENUM_TIMEFRAMES timeframe=ParseTimeframe(configured_timeframes[i]);
      if(timeframe==PERIOD_CURRENT)
        {
         Print("ATOS received unsupported timeframe: ",configured_timeframes[i]);
         return INIT_PARAMETERS_INCORRECT;
        }
      for(int existing=0;existing<ArraySize(g_timeframes);existing++)
         if(g_timeframes[existing]==timeframe)
           {
            Print("ATOS timeframe list contains a duplicate: ",configured_timeframes[i]);
            return INIT_PARAMETERS_INCORRECT;
           }
      int size=ArraySize(g_timeframes);
      if(ArrayResize(g_timeframes,size+1)!=size+1)
         return INIT_FAILED;
      g_timeframes[size]=timeframe;
     }

   int symbols=ArraySize(g_symbols);
   int series_count=symbols*ArraySize(g_timeframes);
   if(ArrayResize(g_last_tick_time,symbols)!=symbols
      || ArrayResize(g_last_tick_signature,symbols)!=symbols
      || ArrayResize(g_last_bar_time,series_count)!=series_count)
      return INIT_FAILED;
   ArrayInitialize(g_last_tick_time,0);
   ArrayInitialize(g_last_bar_time,0);
   for(int i=0;i<symbols;i++)
      g_last_tick_signature[i]="";

   g_account_login=IntegerToString((long)AccountInfoInteger(ACCOUNT_LOGIN));
   g_account_server=AccountInfoString(ACCOUNT_SERVER);
   g_account_currency=AccountInfoString(ACCOUNT_CURRENCY);
   if(g_account_login=="" || g_account_server=="" || g_account_currency=="")
     {
      Print("ATOS could not establish a complete account identity; initialization is blocked.");
      return INIT_FAILED;
     }
   g_client_id=StringFormat("%s@%s",g_account_login,g_account_server);
   // Session identity is intentionally independent of the broker server name length.
   g_session_id=StringFormat("%I64d-%I64u-%u",(long)TimeGMT(),GetMicrosecondCount(),GetTickCount());
   EventSetTimer(InpHeartbeatSeconds);
   g_last_connect_attempt=0;
   Print("ATOS MT5 market-data adapter initialized in READ-ONLY mode with durable broker cursors. Symbols: ",symbols,
      ", timeframes: ",ArraySize(g_timeframes),", session calendar refresh: ",InpSessionCalendarRefreshSeconds,
      " seconds. No trade execution API is available.");
   return INIT_SUCCEEDED;
  }

void OnTimer()
  {
   if(IsStopped())
      return;
   if(!g_authenticated || g_socket==INVALID_HANDLE || !SocketIsConnected(g_socket))
     {
      uint now=GetTickCount();
      if((uint)(now-g_last_connect_attempt)<3000)
         return;
      g_last_connect_attempt=now;
      CloseBridge("");
      if(!ConnectBridge())
         return;
     }
   SendHeartbeat();
  }

void OnDeinit(const int reason)
  {
   EventKillTimer();
   CloseBridge("Expert Advisor stopped, reason "+IntegerToString(reason));
  }
//+------------------------------------------------------------------+
