import type { StructuredReportPresentation } from "./lib/reportPresentation";

type StructuredReportCardProps = {
  presentation: StructuredReportPresentation;
  canShowText: boolean;
};

export function StructuredReportCard({
  presentation,
  canShowText,
}: StructuredReportCardProps) {
  return (
    <article className="structuredReportCard">
      <header className="structuredReportHeader">
        <div>
          <p className="eyebrow">{presentation.eyebrow}</p>
          <h4>{presentation.title}</h4>
        </div>
        <span className="statusPill">{presentation.timeRange}</span>
      </header>

      {presentation.chips.length > 0 ? (
        <div className="reportChipRow" aria-label={`${presentation.title} metadata`}>
          {presentation.chips.map((chip) => (
            <span key={chip}>{chip}</span>
          ))}
        </div>
      ) : null}

      {canShowText ? (
        <>
          <p className="structuredReportLead">{presentation.overview}</p>
          <ol className="reportPhaseList">
            {presentation.phases.map((phase) => (
              <li key={`${phase.label}-${phase.body}`}>
                <strong>{phase.label}</strong>
                <p>{phase.body}</p>
                {phase.meta ? <span>{phase.meta}</span> : null}
              </li>
            ))}
          </ol>

          {presentation.evidence.length > 0 ? (
            <div className="reportEvidenceList" aria-label={`${presentation.title} evidence`}>
              {presentation.evidence.map((item) => (
                <span key={item}>{item}</span>
              ))}
            </div>
          ) : null}

          {presentation.uncertainty.length > 0 ? (
            <div className="reportUncertainty" aria-label={`${presentation.title} uncertainty`}>
              {presentation.uncertainty.map((item) => (
                <span key={item}>{item}</span>
              ))}
            </div>
          ) : null}

          <details className="rawReportDetails">
            <summary>Raw report text</summary>
            <p>{presentation.rawText}</p>
          </details>
        </>
      ) : (
        <p className="redactedText insightRedacted">
          Report narrative generated. Text is hidden in redacted mode.
        </p>
      )}
    </article>
  );
}
