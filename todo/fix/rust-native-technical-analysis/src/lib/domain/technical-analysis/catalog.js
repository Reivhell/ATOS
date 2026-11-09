/** Domain-owned technical-analysis taxonomy and implementation coverage. */

/** @type {Array<{category:string, sections:Array<[string,string]>}>} */
const groups = [
  {
    category: 'SMC & ICT',
    sections: [
      ['Market Structure', 'BOS (Break of Structure)|CHoCH (Change of Character)|MSS (Market Structure Shift)|MSB (Market Structure Break)|CISD (Change in State of Delivery)|HH (Higher High)|HL (Higher Low)|LH (Lower High)|LL (Lower Low)|Internal Structure|External Structure|Protected High|Protected Low|Swing High|Swing Low|Dealing Range'],
      ['Liquidity', 'BSL (Buy-Side Liquidity)|SSL (Sell-Side Liquidity)|EQH (Equal Highs)|EQL (Equal Lows)|Liquidity Sweep|Liquidity Grab|Liquidity Raid|Stop Hunt|Stop Run|Inducement (IDM)|IRL (Internal Range Liquidity)|ERL (External Range Liquidity)|DOL (Draw on Liquidity)|Trendline Liquidity|Session Liquidity|PDH (Previous Day High)|PDL (Previous Day Low)|PWH (Previous Week High)|PWL (Previous Week Low)|PMH (Previous Month High)|PML (Previous Month Low)'],
      ['Blocks', 'OB (Order Block)|BB (Breaker Block)|MB (Mitigation Block)|RB (Rejection Block)|Propulsion Block|Bullish Order Block|Bearish Order Block|Internal Order Block|External Order Block'],
      ['Imbalance & Price Delivery', 'FVG (Fair Value Gap)|IFVG (Inverse Fair Value Gap)|BPR (Balanced Price Range)|VI (Volume Imbalance)|Liquidity Void|Displacement|Inefficiency|Consequent Encroachment (CE)|Rebalancing|Repricing|Partial Fill|Full Fill|Reclaimed FVG'],
      ['Premium, Discount & Fibonacci', 'Premium Zone|Discount Zone|Equilibrium (EQ)|OTE (Optimal Trade Entry)|Fibonacci Retracement|Fibonacci Extension|Fibonacci Projection|Mean Threshold|Midpoint'],
      ['Time & Session', 'Asian Range|London Kill Zone|New York Kill Zone|Judas Swing|Power of Three (PO3 / AMD)|Silver Bullet|ICT Macros|Daily Open|Weekly Open|Monthly Open|True Day Open|Midnight Open|NDOG (New Day Opening Gap)|NWOG (New Week Opening Gap)|Session High|Session Low|Opening Range|CBDR (Central Bank Dealers Range)|MMBM (Market Maker Buy Model)|MMSM (Market Maker Sell Model)|Quarterly Theory'],
      ['Model & Setup ICT', 'Unicorn Model|SMT Divergence|Turtle Soup|Candle Range Theory (CRT)|Market Maker Buy Model|Market Maker Sell Model|Daily Bias|Weekly Bias'],
    ],
  },
  {
    category: 'Price Action',
    sections: [
      ['Market Conditions', 'Trending Market|Ranging Market|Consolidation|Accumulation|Distribution|Expansion|Contraction|Impulse|Correction|Retracement|Pullback|Reversal|Continuation|Breakout|Breakdown|False Breakout|Breakout Retest|Acceptance|Rejection'],
      ['Candlestick Patterns', 'Doji|Dragonfly Doji|Gravestone Doji|Hammer|Inverted Hammer|Hanging Man|Shooting Star|Pin Bar|Marubozu|Spinning Top|Bullish Engulfing|Bearish Engulfing|Harami|Piercing Line|Dark Cloud Cover|Tweezer Top|Tweezer Bottom|Morning Star|Evening Star|Three White Soldiers|Three Black Crows|Inside Bar|Outside Bar|Fakey Pattern|Rising Three Methods|Falling Three Methods|Three-Bar Play'],
      ['Chart Patterns', 'Head & Shoulders|Inverse Head & Shoulders|Double Top|Double Bottom|Triple Top|Triple Bottom|Rounding Top|Rounding Bottom|Rising Wedge|Falling Wedge|Ascending Triangle|Descending Triangle|Symmetrical Triangle|Bull Flag|Bear Flag|Pennant|Rectangle|Cup and Handle|Broadening Formation|Diamond Pattern|Measured Move'],
    ],
  },
  {
    category: 'Support, Resistance & Trend',
    sections: [
      ['Levels & Channels', 'Horizontal Support|Horizontal Resistance|Dynamic Support|Dynamic Resistance|Trendline|Trend Channel|Parallel Channel|Regression Channel|Donchian Channel|Andrews\' Pitchfork|Median Line|Role Reversal|Polarity Change|Flip Zone|Psychological Levels|Round Numbers|Pivot Points|Multi-Timeframe Support / Resistance|Confluence Zone'],
    ],
  },
  {
    category: 'Indikator Teknikal',
    sections: [
      ['Trend', 'SMA (Simple Moving Average)|EMA (Exponential Moving Average)|WMA (Weighted Moving Average)|VWMA (Volume-Weighted Moving Average)|DEMA (Double Exponential Moving Average)|TEMA (Triple Exponential Moving Average)|HMA (Hull Moving Average)|Supertrend|Ichimoku Cloud|Parabolic SAR|ADX (Average Directional Index)|Aroon'],
      ['Momentum', 'RSI (Relative Strength Index)|Stochastic Oscillator|Stochastic RSI|MACD (Moving Average Convergence Divergence)|CCI (Commodity Channel Index)|Williams %R|ROC (Rate of Change)|TRIX|TSI (True Strength Index)|Ultimate Oscillator|Awesome Oscillator|PPO (Percentage Price Oscillator)|RVI (Relative Vigor Index)|Fisher Transform|DPO (Detrended Price Oscillator)|KST (Know Sure Thing)'],
      ['Volatility', 'ATR (Average True Range)|Bollinger Bands|Keltner Channels|Donchian Channels|Standard Deviation|Historical Volatility|Chaikin Volatility|ADR (Average Daily Range)|AWR (Average Weekly Range)|Implied Volatility'],
      ['Volume', 'Volume Bars|Volume Profile|Fixed Range Volume Profile|Visible Range Volume Profile|Session Volume Profile|VWAP (Volume-Weighted Average Price)|Anchored VWAP|OBV (On-Balance Volume)|MFI (Money Flow Index)|CMF (Chaikin Money Flow)|Accumulation / Distribution Line|Volume Oscillator|Volume Delta|Cumulative Volume Delta (CVD)'],
    ],
  },
  {
    category: 'Order Flow & Market Profile',
    sections: [
      ['Auction & Order Flow', 'Market Profile|TPO (Time Price Opportunity)|POC (Point of Control)|VAH (Value Area High)|VAL (Value Area Low)|HVN (High Volume Node)|LVN (Low Volume Node)|Initial Balance|Footprint Chart|Bid-Ask Imbalance|Stacked Imbalance|DOM (Depth of Market)|Level 2 Order Book|Time & Sales|Absorption|Exhaustion|Iceberg Orders|Auction Market Theory|Failed Auction|Market Acceptance|Market Rejection'],
    ],
  },
  {
    category: 'Divergence & Intermarket Analysis',
    sections: [
      ['Divergence & Relationships', 'Regular Bullish Divergence|Regular Bearish Divergence|Hidden Bullish Divergence|Hidden Bearish Divergence|RSI Divergence|MACD Divergence|Stochastic Divergence|Volume Divergence|CVD Divergence|SMT Divergence|Relative Strength|Relative Weakness|Correlation Analysis|Correlation Breakdown|Lead-Lag Relationship'],
    ],
  },
  {
    category: 'Metode Analisis Lain',
    sections: [
      ['Wyckoff', 'Wyckoff Accumulation|Wyckoff Distribution|Spring|Upthrust|UTAD (Upthrust After Distribution)|SOS (Sign of Strength)|SOW (Sign of Weakness)|LPS (Last Point of Support)|LPSY (Last Point of Supply)|Selling Climax|Buying Climax|Automatic Rally|Automatic Reaction|Secondary Test'],
      ['Elliott Wave', 'Impulse Waves 1–5|Corrective Waves A–B–C|Zigzag|Flat Correction|Triangle Correction|Leading Diagonal|Ending Diagonal|Wave Extension'],
      ['Harmonic Patterns', 'Gartley|Bat|Butterfly|Crab|Deep Crab|Shark|Cypher|ABCD Pattern|Three Drives|Potential Reversal Zone (PRZ)'],
      ['Teori Lain', 'Dow Theory|Gann Theory|Mean Reversion|Trend Following|Momentum Trading|Breakout Trading|Range Trading|Statistical Arbitrage|Quantitative Analysis|Seasonal Analysis'],
    ],
  },
  {
    category: 'Fundamental & Makro',
    sections: [
      ['Macro Events & Cross-Market Context', 'Federal Reserve (Fed)|FOMC (Federal Open Market Committee)|Interest Rate|CPI (Consumer Price Index)|PCE (Personal Consumption Expenditures)|NFP (Non-Farm Payrolls)|Unemployment Rate|GDP (Gross Domestic Product)|PMI (Purchasing Managers\' Index)|ISM (Institute for Supply Management)|DXY (US Dollar Index)|US Treasury Yields|Real Yields|Geopolitical Risk|Central Bank Gold Purchases|Gold ETF Flows|Risk-On / Risk-Off|Economic Calendar|News Volatility|Slippage Risk'],
    ],
  },
  {
    category: 'Risk Management & Evaluation',
    sections: [
      ['Execution & Position Risk', 'Entry Trigger|Confirmation Entry|Market Order|Limit Order|Stop Order|Stop Loss (SL)|Take Profit (TP)|Risk-to-Reward Ratio (RR)|Position Sizing|Fixed Fractional Risk|Break-Even (BE)|Trailing Stop|Partial Close|Scaling In|Scaling Out|Maximum Daily Loss|Risk of Ruin|Spread|Commission|Swap|Execution Latency'],
      ['Strategy Validation & Journal', 'Expectancy|Win Rate|Profit Factor|Maximum Drawdown|Backtesting|Forward Testing|Out-of-Sample Testing|Walk-Forward Analysis|Monte Carlo Simulation|Trading Journal'],
    ],
  },
];

const availableCalculations = new Set([
  'SMA (Simple Moving Average)',
  'EMA (Exponential Moving Average)',
  'RSI (Relative Strength Index)',
  'MACD (Moving Average Convergence Divergence)',
  'ATR (Average True Range)',
  'Bollinger Bands',
]);
const availableWorkflows = new Set(['Backtesting']);

/** Strict taxonomy contract: edits to the inventory must update the reviewed snapshot and tests. */
export const TECHNICAL_ANALYSIS_MASTER_LIST_VERSION = '1.0.0';
export const TECHNICAL_ANALYSIS_EXPECTED_ENTRY_COUNT = 364;
export const TECHNICAL_ANALYSIS_EXPECTED_CATEGORY_COUNTS = Object.freeze({
  'SMC & ICT': 97,
  'Price Action': 67,
  'Support, Resistance & Trend': 19,
  'Indikator Teknikal': 52,
  'Order Flow & Market Profile': 21,
  'Divergence & Intermarket Analysis': 15,
  'Metode Analisis Lain': 42,
  'Fundamental & Makro': 20,
  'Risk Management & Evaluation': 31,
});

export const TECHNICAL_ANALYSIS_CATEGORIES = Object.freeze(groups.map((group) => group.category));

export const TECHNICAL_ANALYSIS_CATALOG = Object.freeze(groups.flatMap(({ category, sections }) =>
  sections.flatMap(([group, rawItems]) => rawItems.split('|').map((name) => {
    const available = availableCalculations.has(name) || availableWorkflows.has(name);
    return {
      id: `${category}::${group}::${name}`,
      category,
      group,
      name,
      availability: availableCalculations.has(name)
        ? 'Calculated now'
        : availableWorkflows.has(name)
          ? 'Workflow available'
          : 'Catalog only',
      available,
      kind: availableCalculations.has(name) ? 'indicator' : availableWorkflows.has(name) ? 'workflow' : 'research concept',
    };
  })).map((entry) => Object.freeze(entry))));

