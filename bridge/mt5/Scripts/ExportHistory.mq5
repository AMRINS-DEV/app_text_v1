//+------------------------------------------------------------------+
//| One-off script to export MT5 history bars for backtest seeding   |
//| (feeds crates/storage's QuestDB writer / NautilusTrader). Phase   |
//| 1 scope.                                                          |
//+------------------------------------------------------------------+
#property strict
#property script_show_inputs

void OnStart()
{
   // Phase 1: iterate CopyRates() over the configured symbol/timeframe
   // range and write CSV/binary output for the storage pipeline to ingest.
}
