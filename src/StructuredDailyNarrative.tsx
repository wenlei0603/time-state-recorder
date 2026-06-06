import { presentDailyNarrative } from "./lib/reportPresentation";
import { StructuredReportCard } from "./StructuredReportCard";
import type { DailyBrief } from "./types";

type StructuredDailyNarrativeProps = {
  brief: DailyBrief;
  canShowText: boolean;
};

export function StructuredDailyNarrative({
  brief,
  canShowText,
}: StructuredDailyNarrativeProps) {
  return (
    <StructuredReportCard
      presentation={presentDailyNarrative(brief)}
      canShowText={canShowText}
    />
  );
}
