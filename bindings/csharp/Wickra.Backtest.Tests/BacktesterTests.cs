using System;
using System.Text.Json;
using Wickra.Backtest;
using Xunit;

public class BacktesterTests
{
    private const string PriceSpec =
        "{\"symbol\":\"x\",\"timeframe\":\"1h\",\"indicators\":{}," +
        "\"entry\":{\"gt\":[{\"price\":\"close\"},100]}," +
        "\"exit\":{\"lt\":[{\"price\":\"close\"},100]}," +
        "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

    [Fact]
    public void VersionIsNonEmpty()
    {
        Assert.False(string.IsNullOrEmpty(Backtester.Version()));
    }

    [Fact]
    public void HandComputedRoundTripMatchesEngine()
    {
        var open = new[] { 100.0, 102.0, 104.0, 98.0 };
        var high = new[] { 101.0, 103.0, 104.0, 98.0 };
        var low = new[] { 100.0, 102.0, 99.0, 97.0 };
        var close = new[] { 101.0, 103.0, 99.0, 97.0 };
        var time = new long[] { 0, 1, 2, 3 };

        string json = Backtester.Run(open, high, low, close, time: time, spec: PriceSpec, capital: 1000.0);
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;

        Assert.Equal(1, root.GetProperty("metrics").GetProperty("num_trades").GetInt32());
        var trade = root.GetProperty("trades")[0];
        Assert.True(Math.Abs(trade.GetProperty("entry_price").GetDouble() - 102.0) < 1e-9);
        Assert.True(Math.Abs(trade.GetProperty("exit_price").GetDouble() - 98.0) < 1e-9);
        Assert.True(Math.Abs(trade.GetProperty("pnl").GetDouble() - (-4.0)) < 1e-9);

        var equity = root.GetProperty("equity");
        var last = equity[equity.GetArrayLength() - 1];
        Assert.True(Math.Abs(last.GetProperty("equity").GetDouble() - 996.0) < 1e-9);
    }

    [Fact]
    public void RunJsonRequestBundle()
    {
        string request =
            "{\"capital\":1000,\"spec\":" + PriceSpec + ",\"candles\":[" +
            "{\"time\":0,\"open\":100,\"high\":101,\"low\":100,\"close\":101}," +
            "{\"time\":1,\"open\":102,\"high\":103,\"low\":102,\"close\":103}," +
            "{\"time\":2,\"open\":104,\"high\":104,\"low\":99,\"close\":99}," +
            "{\"time\":3,\"open\":98,\"high\":98,\"low\":97,\"close\":97}]}";

        string json = Backtester.RunJson(request);
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;

        Assert.Equal(1, root.GetProperty("metrics").GetProperty("num_trades").GetInt32());
        var trade = root.GetProperty("trades")[0];
        Assert.True(Math.Abs(trade.GetProperty("entry_price").GetDouble() - 102.0) < 1e-9);
    }

    [Fact]
    public void InvalidSpecThrows()
    {
        var one = new[] { 1.0 };
        Assert.Throws<InvalidOperationException>(() =>
            Backtester.Run(one, one, one, one, spec: "{\"bad\":true}"));
    }
}
