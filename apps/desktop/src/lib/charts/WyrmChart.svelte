<script lang="ts">
  import { BarChart, LineChart } from "echarts/charts";
  import {
    GridComponent,
    LegendComponent,
    MarkLineComponent,
    TooltipComponent,
  } from "echarts/components";
  import * as echarts from "echarts/core";
  import { CanvasRenderer } from "echarts/renderers";
  import type { ECharts, EChartsCoreOption } from "echarts/core";
  import { onMount } from "svelte";
  import { activeTheme } from "$lib/theme/runtime";
  import { formatLocalDateTime } from "$lib/presentation/dateTime";
  import type { ThemeManifest } from "$lib/theme/types";
  import {
    categoricalChartHeight,
    readableCategoryLabel,
    usesHorizontalBars,
  } from "./layout";
  import { chartPresentation } from "./theme";
  import type { ChartSpec } from "./types";

  echarts.use([
    BarChart,
    LineChart,
    GridComponent,
    LegendComponent,
    MarkLineComponent,
    TooltipComponent,
    CanvasRenderer,
  ]);

  let {
    spec,
    height = "180px",
    eyebrow = "WyrmGrid Hoard",
  }: {
    spec: ChartSpec;
    height?: string;
    eyebrow?: string;
  } = $props();

  let container = $state<HTMLDivElement>();
  let chart = $state.raw<ECharts>();
  const categories = $derived(expandedCategories(spec.series[0]?.points ?? []));
  const resolvedHeight = $derived(
    usesHorizontalBars(spec.kind, categories)
      ? categoricalChartHeight(categories.length)
      : height,
  );

  const numberFormatter = new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 2,
  });

  function formatValue(value: number): string {
    return `${numberFormatter.format(value)}${spec.unit ? ` ${spec.unit}` : ""}`;
  }

  function formatObservedAt(value: string): string {
    return formatLocalDateTime(value, "Observation time unavailable");
  }

  function expandedCategories(
    points: Array<{ category: string; gap_before?: boolean }>,
  ): string[] {
    return points.flatMap((point) =>
      point.gap_before
        ? [`${point.category} · gap`, point.category]
        : [point.category],
    );
  }

  function expandedValues(
    points: Array<{ value: number; gap_before?: boolean }>,
  ): Array<number | null> {
    return points.flatMap((point) =>
      point.gap_before ? [null, point.value] : [point.value],
    );
  }

  function optionFor(
    chartSpec: ChartSpec,
    theme: ThemeManifest,
  ): EChartsCoreOption {
    const categories = expandedCategories(chartSpec.series[0]?.points ?? []);
    const presentation = chartPresentation(theme);
    const horizontalBars = usesHorizontalBars(chartSpec.kind, categories);
    const categoryAxis = {
      type: "category",
      name: horizontalBars ? undefined : chartSpec.category_axis_label,
      nameLocation: "middle",
      nameGap: 24,
      data: categories,
      boundaryGap: chartSpec.kind === "bar",
      inverse: horizontalBars,
      axisLine: { lineStyle: { color: presentation.line } },
      axisTick: { show: false },
      axisLabel: {
        color: presentation.muted,
        interval: chartSpec.kind === "bar" ? 0 : undefined,
        rotate: horizontalBars ? 0 : chartSpec.kind === "bar" ? 20 : 0,
        formatter:
          chartSpec.kind === "bar"
            ? (value: string) => readableCategoryLabel(value)
            : undefined,
      },
      nameTextStyle: { color: presentation.muted, fontSize: 10 },
    };
    const valueAxis = {
      type: "value",
      name: chartSpec.value_axis_label,
      nameLocation: horizontalBars ? "middle" : undefined,
      nameGap: horizontalBars ? 24 : undefined,
      nameTextStyle: { color: presentation.muted, fontSize: 10 },
      splitLine: { lineStyle: { color: presentation.line } },
      axisLabel: { color: presentation.muted },
    };

    return {
      animationDuration: 550,
      color: presentation.colours,
      grid: {
        left: horizontalBars ? 130 : 44,
        right: 12,
        top: chartSpec.series.length > 1 ? 28 : 12,
        bottom: horizontalBars ? 32 : chartSpec.kind === "bar" ? 58 : 30,
      },
      legend: {
        show: chartSpec.series.length > 1,
        top: 0,
        textStyle: { color: presentation.muted },
      },
      tooltip: {
        trigger: "axis",
        backgroundColor: presentation.tooltipBackground,
        borderColor: presentation.tooltipBorder,
        textStyle: { color: presentation.text },
        valueFormatter: (value: unknown) =>
          typeof value === "number" ? formatValue(value) : String(value ?? ""),
      },
      xAxis: horizontalBars ? valueAxis : categoryAxis,
      yAxis: horizontalBars ? categoryAxis : valueAxis,
      series: chartSpec.series.map((series, index) => ({
        id: series.id,
        name: series.label,
        type: chartSpec.kind === "bar" ? "bar" : "line",
        data: expandedValues(series.points),
        connectNulls: false,
        smooth: chartSpec.kind !== "bar",
        symbol: "circle",
        symbolSize: 6,
        lineStyle: { width: 2 },
        areaStyle: chartSpec.kind === "area" ? { opacity: 0.16 } : undefined,
        markLine:
          index === 0 && chartSpec.reference_lines?.length
            ? {
                silent: true,
                symbol: ["none", "none"],
                label: {
                  color: presentation.muted,
                  formatter: "{b}",
                },
                lineStyle: {
                  color: presentation.muted,
                  type: "dashed",
                  width: 1,
                },
                data: chartSpec.reference_lines.map((line) => ({
                  id: line.id,
                  name: line.label,
                  ...(line.axis === "category"
                    ? { xAxis: line.value }
                    : { yAxis: line.value }),
                })),
              }
            : undefined,
      })),
    };
  }

  $effect(() => {
    if (chart)
      chart.setOption(optionFor(spec, $activeTheme), { notMerge: true });
  });

  onMount(() => {
    if (!container) return;

    const chartContainer = container;
    chart = echarts.init(chartContainer, undefined, { renderer: "canvas" });
    chart.setOption(optionFor(spec, $activeTheme));

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
      <span class="chart-eyebrow">{eyebrow}</span>
      <h3>{spec.title}</h3>
    </div>
    <span class="provenance" data-kind={spec.provenance.kind}
      >{spec.provenance.kind.replace("_", " ")}</span
    >
  </header>

  {#if spec.description}<p>{spec.description}</p>{/if}

  {#if spec.series.length === 0}
    <div class="chart-state" style:height={resolvedHeight}>
      No data available
    </div>
  {:else}
    <div
      bind:this={container}
      class="chart"
      style:height={resolvedHeight}
      role="img"
      aria-label={`${spec.title}. ${spec.description ?? ""}`}
    ></div>
    <div class="screen-reader-summary">
      {#each spec.series as series}
        {series.label}: {series.points
          .map(
            (point) =>
              `${point.gap_before ? "gap before, " : ""}${point.category}, ${formatValue(point.value)}`,
          )
          .join("; ")}.
      {/each}
      {#each spec.reference_lines ?? [] as line}
        {line.label}: {typeof line.value === "number"
          ? formatValue(line.value)
          : line.value}.
      {/each}
    </div>
  {/if}

  <footer>
    <span>{spec.provenance.source}</span>
    <time datetime={spec.provenance.observed_at}
      >{formatObservedAt(spec.provenance.observed_at)}</time
    >
  </footer>
</article>

<style>
  .chart-card {
    margin-top: 16px;
    padding-top: 14px;
    border-top: 1px solid var(--color-line-faint);
  }
  header,
  footer {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .chart-eyebrow {
    color: var(--color-highlight);
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
    color: var(--color-text-muted);
    font-size: 11px;
    line-height: 1.45;
  }
  .provenance {
    padding: 4px 6px;
    border: 1px solid var(--color-highlight-border);
    border-radius: 3px;
    color: var(--color-highlight);
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
    color: var(--color-text-muted);
    font-size: 12px;
  }
  footer {
    align-items: center;
    margin-top: 4px;
    color: var(--color-text-muted);
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
