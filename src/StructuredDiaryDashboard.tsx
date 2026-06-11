import { BriefcaseBusiness, Clock3, MapPin, Shuffle, Users } from "lucide-react";
import type { ReactNode } from "react";
import type {
  DiaryDashboard,
  DiaryDashboardEntity,
  DiaryDashboardRole,
  DiaryDashboardTeam,
  DiaryDashboardTimeBlock,
} from "./types";

type StructuredDiaryDashboardProps = {
  dashboard: DiaryDashboard;
  canShowText: boolean;
};

export function StructuredDiaryDashboard({
  dashboard,
  canShowText,
}: StructuredDiaryDashboardProps) {
  return (
    <div className="diaryDashboard">
      <div className="diaryDashboardStats" aria-label="Diary dashboard metrics">
        <DashboardMetric
          icon={<Users size={16} />}
          value={`${dashboard.teamCount} teams`}
          label="team spread"
        />
        <DashboardMetric
          icon={<Shuffle size={16} />}
          value={dashboard.roleHeterogeneity.level || "unknown"}
          label="role mix"
        />
        <DashboardMetric
          icon={<BriefcaseBusiness size={16} />}
          value={`${dashboard.workTypes.length} types`}
          label="work types"
        />
        <DashboardMetric
          icon={<Clock3 size={16} />}
          value={formatDuration(totalActiveSeconds(dashboard.timeDistribution))}
          label="distributed"
        />
      </div>

      {canShowText ? (
        <>
          <p className="diaryDashboardOverview">{dashboard.overview}</p>
          <div className="diaryDashboardGrid">
            <DashboardList
              title="Teams"
              items={dashboard.teams}
              renderItem={(team) => (
                <TeamRow key={team.name} team={team} />
              )}
            />
            <DashboardList
              title="Roles"
              items={dashboard.roles}
              renderItem={(role) => (
                <RoleRow key={role.label} role={role} />
              )}
            />
            <DashboardList
              title="Work Types"
              items={dashboard.workTypes}
              renderItem={(item) => (
                <EntityRow key={item.label} item={item} />
              )}
            />
            <DashboardList
              title="Places"
              items={dashboard.locations}
              renderItem={(item) => (
                <EntityRow key={item.label} item={item} icon={<MapPin size={14} />} />
              )}
            />
            <DashboardList
              title="Collaborators"
              items={dashboard.collaborators}
              renderItem={(item) => (
                <EntityRow key={item.label} item={item} />
              )}
            />
            <DashboardList
              title="Time Distribution"
              items={dashboard.timeDistribution}
              renderItem={(block) => (
                <TimeBlockRow key={`${block.label}-${block.startAt ?? ""}`} block={block} />
              )}
            />
          </div>

          <div className="diaryDashboardNotes">
            {dashboard.roleHeterogeneity.summary ? (
              <span>{dashboard.roleHeterogeneity.summary}</span>
            ) : null}
            {dashboard.evidenceSummary ? <span>{dashboard.evidenceSummary}</span> : null}
            {dashboard.uncertainty ? <span>{dashboard.uncertainty}</span> : null}
          </div>
        </>
      ) : (
        <p className="redactedText insightRedacted">
          Structured diary dashboard generated. Text is hidden in redacted mode.
        </p>
      )}
    </div>
  );
}

function DashboardMetric({
  icon,
  value,
  label,
}: {
  icon: ReactNode;
  value: string;
  label: string;
}) {
  return (
    <div className="diaryDashboardMetric">
      <span className="metricIcon">{icon}</span>
      <div>
        <strong>{value}</strong>
        <span>{label}</span>
      </div>
    </div>
  );
}

function DashboardList<T>({
  title,
  items,
  renderItem,
}: {
  title: string;
  items: T[];
  renderItem: (item: T) => ReactNode;
}) {
  return (
    <section className="diaryDashboardGroup" aria-label={title}>
      <h4>{title}</h4>
      {items.length > 0 ? (
        <div className="diaryDashboardRows">{items.map(renderItem)}</div>
      ) : (
        <p className="emptyState">No reliable signal.</p>
      )}
    </section>
  );
}

function EntityRow({
  item,
  icon,
}: {
  item: DiaryDashboardEntity;
  icon?: ReactNode;
}) {
  return (
    <div className="diaryDashboardRow">
      <div>
        <strong>
          {icon}
          {item.label}
        </strong>
        {item.description ? <p>{item.description}</p> : null}
      </div>
      <span>{formatShareAndDuration(item.share, item.activeSeconds)}</span>
    </div>
  );
}

function RoleRow({ role }: { role: DiaryDashboardRole }) {
  return (
    <div className="diaryDashboardRow">
      <div>
        <strong>{role.label}</strong>
        {role.description ? <p>{role.description}</p> : null}
      </div>
      <span>{formatShareAndDuration(role.share, role.activeSeconds)}</span>
    </div>
  );
}

function TeamRow({ team }: { team: DiaryDashboardTeam }) {
  return (
    <div className="diaryDashboardRow">
      <div>
        <strong>{team.name}</strong>
        <p>
          {[team.role, ...team.workTypes].filter(Boolean).join(" / ") || "Unclassified"}
        </p>
      </div>
      <span>{formatShareAndDuration(team.share, team.activeSeconds)}</span>
    </div>
  );
}

function TimeBlockRow({ block }: { block: DiaryDashboardTimeBlock }) {
  return (
    <div className="diaryDashboardRow">
      <div>
        <strong>{block.label}</strong>
        <p>{block.summary || [block.primaryTeam, block.primaryWorkType, block.role].filter(Boolean).join(" / ")}</p>
      </div>
      <span>{formatDuration(block.activeSeconds)}</span>
    </div>
  );
}

function totalActiveSeconds(blocks: DiaryDashboardTimeBlock[]): number {
  return blocks.reduce((total, block) => total + block.activeSeconds, 0);
}

function formatShareAndDuration(share?: number, seconds?: number): string {
  const parts = [];
  if (typeof share === "number") {
    parts.push(`${Math.round(share * 100)}%`);
  }
  if (typeof seconds === "number") {
    parts.push(formatDuration(seconds));
  }
  return parts.join(" · ") || "inferred";
}

function formatDuration(seconds: number): string {
  if (seconds <= 0) return "0m";
  if (seconds >= 3600) return `${(seconds / 3600).toFixed(1)}h`;
  return `${Math.round(seconds / 60)}m`;
}
