using System;
using System.Text.Json;
using Wickra.Backtest;
using Xunit;

// The streaming class must be the same engine as Backtester.Run, one bar at a
// time -- that equivalence is the claim, so it is what these tests pin.
public class StreamingBacktestTests
{
    private const string PriceSpec =
        "{\"symbol\":\"x\",\"timeframe\":\"1h\",\"indicators\":{}," +
        "\"entry\":{\"gt\":[{\"price\":\"close\"},100]}," +
        "\"exit\":{\"lt\":[{\"price\":\"close\"},100]}," +
        "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

    private static readonly double[] Open = { 100.0, 102.0, 104.0, 98.0 };
    private static readonly double[] High = { 101.0, 103.0, 104.0, 98.0 };
    private static readonly double[] Low = { 100.0, 102.0, 99.0, 97.0 };
    private static readonly double[] Close = { 101.0, 103.0, 99.0, 97.0 };

    private static string BatchReport() =>
        Backtester.Run(Open, High, Low, Close, spec: PriceSpec, capital: 1000.0);

    [Fact]
    public void StreamingReproducesTheBatchReport()
    {
        using var bt = new StreamingBacktest(PriceSpec, 1000.0);
        for (int i = 0; i < Open.Length; i++)
        {
            bt.Step(Open[i], High[i], Low[i], Close[i]);
        }
        Assert.Equal(BatchReport(), bt.FinishJson());
    }

    [Fact]
    public void StepJsonMatchesTheScalarStep()
    {
        using var bt = new StreamingBacktest(PriceSpec, 1000.0);
        for (int i = 0; i < Open.Length; i++)
        {
            string doc =
                "{\"candle\":{\"time\":" + i +
                ",\"open\":" + Open[i] + ",\"high\":" + High[i] +
                ",\"low\":" + Low[i] + ",\"close\":" + Close[i] + ",\"volume\":0}}";
            bt.StepJson(doc);
        }
        Assert.Equal(BatchReport(), bt.FinishJson());
    }

    [Fact]
    public void AccessorsTrackTheRun()
    {
        using var bt = new StreamingBacktest(PriceSpec, 1000.0);
        Assert.Equal("null", bt.LatestEquityJson());
        Assert.Equal("[]", bt.EquityJson());
        Assert.Equal(0, bt.NumTrades);
        Assert.False(bt.IsFinished);

        for (int i = 0; i < 3; i++)
        {
            bt.Step(Open[i], High[i], Low[i], Close[i]);
        }

        using var curve = JsonDocument.Parse(bt.EquityJson());
        Assert.Equal(3, curve.RootElement.GetArrayLength());
        // Bar 2 closed below 100, which is the exit *signal*; the fill lands on
        // the next bar's open, so nothing has closed yet.
        Assert.Equal(0, bt.NumTrades);

        bt.Step(Open[3], High[3], Low[3], Close[3]);
        Assert.Equal(1, bt.NumTrades);
    }

    [Fact]
    public void TimeDefaultsToTheBarIndex()
    {
        using var bt = new StreamingBacktest(PriceSpec, 1000.0);
        for (int i = 0; i < Open.Length; i++)
        {
            bt.Step(Open[i], High[i], Low[i], Close[i]);
        }
        using var curve = JsonDocument.Parse(bt.EquityJson());
        int index = 0;
        foreach (var point in curve.RootElement.EnumerateArray())
        {
            Assert.Equal(index, point.GetProperty("time").GetInt64());
            index++;
        }
        Assert.Equal(Open.Length, index);
    }

    [Fact]
    public void AFinishedRunRefusesFurtherUse()
    {
        var bt = new StreamingBacktest(PriceSpec, 1000.0);
        bt.Step(Open[0], High[0], Low[0], Close[0]);
        bt.FinishJson();
        Assert.True(bt.IsFinished);

        Assert.Throws<ObjectDisposedException>(() => bt.Step(Open[1], High[1], Low[1], Close[1]));
        Assert.Throws<ObjectDisposedException>(() => bt.EquityJson());
        Assert.Throws<ObjectDisposedException>(() => bt.LatestEquityJson());
        Assert.Throws<ObjectDisposedException>(() => bt.FinishJson());
        Assert.Throws<ObjectDisposedException>(() => bt.NumTrades);
    }

    [Fact]
    public void DisposeIsIdempotentAndEndsTheRun()
    {
        var bt = new StreamingBacktest(PriceSpec, 1000.0);
        bt.Step(Open[0], High[0], Low[0], Close[0]);
        bt.Dispose();
        bt.Dispose();
        Assert.True(bt.IsFinished);
    }

    [Fact]
    public void AnInvalidSpecThrows()
    {
        Assert.Throws<InvalidOperationException>(() => new StreamingBacktest("{\"bad\":true}"));
    }

    [Fact]
    public void PerBarFeedsReachAReferenceReadingStrategy()
    {
        // A sine path, not a geometric one: constant growth means constant log
        // returns, which drives the correlation's variance to zero.
        const int N = 24;
        var closes = new double[N];
        for (int i = 0; i < N; i++)
        {
            closes[i] = 100.0 + 10.0 * Math.Sin(i * 0.5);
        }

        const string Spec =
            "{\"symbol\":\"x\",\"timeframe\":\"1h\"," +
            "\"indicators\":{\"corr\":{\"type\":\"PearsonCorrelation\",\"params\":[5]}}," +
            "\"entry\":{\"gt\":[\"corr\",0.5]},\"exit\":{\"lt\":[\"corr\",-0.5]}," +
            "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

        string streamed;
        using (var bt = new StreamingBacktest(Spec, 1000.0))
        {
            for (int i = 0; i < N; i++)
            {
                double c = closes[i];
                bt.StepJson(
                    "{\"candle\":{\"time\":" + i + ",\"open\":" + c +
                    ",\"high\":" + (c + 1.0) + ",\"low\":" + (c - 1.0) +
                    ",\"close\":" + c + ",\"volume\":0}," +
                    "\"feeds\":{\"reference\":" + (2.0 * c) + "}}");
            }
            streamed = bt.FinishJson();
        }

        using var streamedDoc = JsonDocument.Parse(streamed);
        Assert.Equal(
            1,
            streamedDoc.RootElement.GetProperty("metrics").GetProperty("num_trades").GetInt32());

        // The feed is load-bearing: without it the correlation never resolves,
        // so the strategy never fires and the two runs cannot agree.
        string blind;
        using (var bt = new StreamingBacktest(Spec, 1000.0))
        {
            for (int i = 0; i < N; i++)
            {
                double c = closes[i];
                bt.Step(c, c + 1.0, c - 1.0, c);
            }
            blind = bt.FinishJson();
        }

        using var blindDoc = JsonDocument.Parse(blind);
        Assert.Equal(
            0,
            blindDoc.RootElement.GetProperty("metrics").GetProperty("num_trades").GetInt32());
        Assert.NotEqual(blind, streamed);
    }
}
