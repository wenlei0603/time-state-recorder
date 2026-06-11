import type { ECharts, EChartsOption } from "echarts";
import { useEffect, useRef } from "react";

type EChartPanelProps = {
  title: string;
  option: EChartsOption;
  className?: string;
};

export function EChartPanel({ title, option, className = "" }: EChartPanelProps) {
  const chartElement = useRef<HTMLDivElement | null>(null);
  const chartInstance = useRef<ECharts | null>(null);
  const latestOption = useRef(option);
  latestOption.current = option;

  useEffect(() => {
    if (!chartElement.current) {
      return;
    }
    let cancelled = false;
    let instance: ECharts | null = null;
    const resize = () => chartInstance.current?.resize();

    void import("echarts").then((echarts) => {
      if (cancelled || !chartElement.current) {
        return;
      }
      instance = echarts.init(chartElement.current, undefined, {
        renderer: "canvas",
      });
      chartInstance.current = instance;
      chartInstance.current.setOption(latestOption.current, true);
      window.addEventListener("resize", resize);
    });

    return () => {
      cancelled = true;
      window.removeEventListener("resize", resize);
      instance?.dispose();
      chartInstance.current = null;
    };
  }, []);

  useEffect(() => {
    chartInstance.current?.setOption(option, true);
  }, [option]);

  return (
    <section className={`chartPanel ${className}`} aria-label={title}>
      <div className="panelHeader">
        <h3>{title}</h3>
      </div>
      <div className="chartCanvas" ref={chartElement} role="img" aria-label={`${title} chart`} />
    </section>
  );
}
