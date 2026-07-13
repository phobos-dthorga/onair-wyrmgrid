<script lang="ts">
  import { BarChart, LineChart } from "echarts/charts";
  import {
    GridComponent,
    LegendComponent,
    TooltipComponent,
  } from "echarts/components";
  import * as echarts from "echarts/core";
  import { CanvasRenderer } from "echarts/renderers";
  import type { ECharts, EChartsCoreOption } from "echarts/core";
  import { onMount } from "svelte";
  import { chartColours, chartTheme } from "./theme";
  import type { ChartSpec } from "./types";

  echarts.use([
    BarChart,
    LineChart,
    GridComponent,
    LegendComponent,
    TooltipComponent,
    CanvasRenderer,
  ]);

  let {
    spec,
    height = "180px",
  }: {
    spec: ChartSpec;
    height?: string;
  } = $props();

  let container = $state<HTMLDivElement>();
  let chart = $state.raw<ECharts>();

  const numberFormatter = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 });

  function formatValue(value: number): string {
    return `${numberFormatter.format(value)}${spec.unit ? ` ${spec.unit}` : ""}`;
  }

  function optionFor(chartSpec: ChartSpec): EChartsCoreOption {
    const categories = chartSpec.series[0]?.points.map((point) => point.category) ?? [];

    return {
      animationDuration: 550,
      color: chartColours,
      grid: { left: 44, right: 12, top: chartSpec.series.length > 1 ? 28 : 12, bottom: 30 },
      legend: {
        show: chartSpec.series.length > 1,
        top: 0,
        textStyle: { color: chartTheme.muted },
      },
      tooltip: {
        trigger: "axis",
        backgroundColor: chartTheme.tooltipBackground,
        borderColor: chartTheme.tooltipBorder,
        textStyle: { color: chartTheme.text },
        valueFormatter: (value: unknown) =>
          typeof value === "number" ? formatValue(value) : String(value ?? ""),
      },
      xAxis: {
        type: "category",
        name: chartSpec.category_axis_label,
        nameLocation: "middle",
        nameGap: 24,
        data: categories,
        boundaryGap: chartSpec.kind === "bar",
        axisLine: { lineStyle: { color: chartTheme.line } },
        axisTick: { show: false },
        axisLabel: { color: chartTheme.muted },
        nameTextStyle: { color: chartTheme.muted, fontSize: 10 },
      },
      yAxis: {
        type: "value",
        name: chartSpec.value_axis_label,
        nameTextStyle: { color: chartTheme.muted, fontSize: 10 },
        splitLine: { lineStyle: { color: chartTheme.line } },
        axisLabel: { color: chartTheme.muted },
      },
      series: chartSpec.series.map((series) => ({
        id: series.id,
        name: series.label,
        type: chartSpec.kind === "bar" ? "bar" : "line",
        data: series.points.map((point) => point.value),
        smooth: chartSpec.kind !== "bar",
        symbol: "circle",
        symbolSize: 6,
        lineStyle: { width: 2 },
        areaStyle: chartSpec.kind === "area" ? { opacity: 0.16 } : undefined,
      })),
    };
  }

  $effect(() => {
    if (chart) chart.setOption(optionFor(spec), { notMerge: true });
  });

  onMount(() => {
    if (!container) return;

    const chartContainer = container;
    chart = echarts.init(chartContainer, undefined, { renderer: "canvas" });
    chart.setOption(optionFor(spec));

    const resizeObserver = new ResizeObserver(() => chart?.resize());
    resizeObserver.observe(chartContainer);

    return () => {
      resizeObserver.disconnect();
      chart?.dispose();
      chart = undefined;
    };
  });
</script>

<article class="chart-card">
  <header>
    <div>
      <span class="chart-eyebrow">WyrmGrid Hoard</span>
      <h3>{spec.title}</h3>
    </div>
    <span class="provenance" data-kind={spec.provenance.kind}>{spec.provenance.kind.replace("_", " ")}</span>
  </header>

  {#if spec.description}<p>{spec.description}</p>{/if}

  {#if spec.series.length === 0}
    <div class="chart-state" style:height>No data available</div>
  {:else}
    <div
      bind:this={container}
      class="chart"
      style:height
      role="img"
      aria-label={`${spec.title}. ${spec.description ?? ""}`}
    ></div>
    <div class="screen-reader-summary">
      {#each spec.series as series}
        {series.label}: {series.points
          .map((point) => `${point.category}, ${formatValue(point.value)}`)
          .join("; ")}.
      {/each}
    </div>
  {/if}

  <footer>
    <span>{spec.provenance.source}</span>
    <time datetime={spec.provenance.observed_at}>Illustrative · not live OnAir data</time>
  </footer>
</article>

<style>
  .chart-card {
    margin-top: 16px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }
  header,
  footer {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .chart-eyebrow {
    color: var(--gold);
    text-transform: uppercase;
    letter-spacing: 0.14em;
    font-size: 9px;
    font-weight: 700;
  }
  h3 {
    margin: 4px 0 0;
    font-family: Georgia, serif;
    font-size: 18px;
    font-weight: 500;
  }
  p {
    margin: 7px 0 0;
    color: var(--muted);
    font-size: 11px;
    line-height: 1.45;
  }
  .provenance {
    padding: 4px 6px;
    border: 1px solid rgba(213, 174, 95, 0.28);
    border-radius: 3px;
    color: var(--gold);
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .chart {
    width: 100%;
    min-height: 120px;
    margin-top: 8px;
  }
  .chart-state {
    display: grid;
    place-items: center;
    color: var(--muted);
    font-size: 12px;
  }
  footer {
    align-items: center;
    margin-top: 4px;
    color: #6f8580;
    font-size: 8px;
  }
  .screen-reader-summary {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
