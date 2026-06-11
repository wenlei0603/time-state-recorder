use std::collections::{BTreeMap, BTreeSet};

use crate::{
    models::{
        DailyActivityStats, DailyBrief, DiaryDashboard, DiaryDashboardEntity, DiaryDashboardRole,
        DiaryDashboardTeam, DiaryDashboardTimeBlock, DiaryRoleHeterogeneity, HourlyActivityMetric,
        InsightReport,
    },
    prompt_time::{human_report_range, human_report_timestamp},
};

pub fn build_diary_dashboard(
    brief: Option<&DailyBrief>,
    stats: &DailyActivityStats,
    hourly_metrics: &[HourlyActivityMetric],
    five_hour_reports: &[InsightReport],
) -> DiaryDashboard {
    if let Some(dashboard) = brief
        .and_then(|brief| dashboard_from_model_json(&brief.raw_summary_json))
        .filter(has_dashboard_content)
    {
        return normalize_dashboard(dashboard);
    }

    build_fallback_dashboard(brief, stats, hourly_metrics, five_hour_reports)
}

fn dashboard_from_model_json(value: &serde_json::Value) -> Option<DiaryDashboard> {
    [
        "diaryDashboard",
        "structuredDiaryDashboard",
        "workOverviewDashboard",
    ]
    .iter()
    .find_map(|key| value.get(*key))
    .and_then(|value| serde_json::from_value(value.clone()).ok())
}

fn normalize_dashboard(mut dashboard: DiaryDashboard) -> DiaryDashboard {
    dashboard
        .collaborators
        .retain(|item| !item.label.trim().is_empty());
    dashboard
        .locations
        .retain(|item| !item.label.trim().is_empty());
    dashboard
        .work_types
        .retain(|item| !item.label.trim().is_empty());
    dashboard.roles.retain(|item| !item.label.trim().is_empty());
    dashboard.teams.retain(|item| !item.name.trim().is_empty());
    dashboard
        .time_distribution
        .retain(|item| !item.label.trim().is_empty() || item.active_seconds > 0);
    if dashboard.team_count == 0 {
        dashboard.team_count = dashboard.teams.len();
    }
    if dashboard.role_heterogeneity.level.trim().is_empty() {
        dashboard.role_heterogeneity = role_heterogeneity(&dashboard.roles);
    }
    dashboard
}

fn has_dashboard_content(dashboard: &DiaryDashboard) -> bool {
    !dashboard.overview.trim().is_empty()
        || dashboard.team_count > 0
        || !dashboard.teams.is_empty()
        || !dashboard.work_types.is_empty()
        || !dashboard.roles.is_empty()
}

fn build_fallback_dashboard(
    brief: Option<&DailyBrief>,
    stats: &DailyActivityStats,
    hourly_metrics: &[HourlyActivityMetric],
    five_hour_reports: &[InsightReport],
) -> DiaryDashboard {
    let locations = location_distribution(stats);
    let work_types = work_type_distribution(stats, hourly_metrics);
    let teams = team_distribution(stats, five_hour_reports);
    let roles = role_distribution(&work_types, &teams);
    let time_distribution = time_blocks(hourly_metrics, &teams);
    let role_heterogeneity = role_heterogeneity(&roles);

    DiaryDashboard {
        overview: overview_text(brief, stats, teams.len(), roles.len()),
        collaborators: vec![DiaryDashboardEntity {
            label: "Unknown collaborators".into(),
            description: "窗口和统计数据不足以可靠识别具体协作者；需要依赖 MiniMax daily brief 结构化输出。".into(),
            active_seconds: None,
            share: None,
            evidence: vec!["fallback_from_activity_metrics".into()],
            confidence: Some(0.2),
        }],
        locations,
        work_types,
        roles,
        role_heterogeneity,
        team_count: teams.len(),
        teams,
        time_distribution,
        evidence_summary: "Fallback dashboard built from descriptive stats, hourly metrics, and 5h reports.".into(),
        uncertainty: "该 fallback 只能概括工作类型、应用和项目线索；具体人员、真实地点和团队边界需要 MiniMax-M3 模型从 daily brief 证据中推断。".into(),
    }
}

fn overview_text(
    brief: Option<&DailyBrief>,
    stats: &DailyActivityStats,
    team_count: usize,
    role_count: usize,
) -> String {
    if let Some(text) = brief
        .map(|brief| brief.daily_summary_text.trim())
        .filter(|text| !text.is_empty())
    {
        return text.to_string();
    }
    format!(
        "当天记录到 {:.1} 小时活跃时间，覆盖 {} 个团队线索和 {} 类角色。",
        stats.active_hours, team_count, role_count
    )
}

fn location_distribution(stats: &DailyActivityStats) -> Vec<DiaryDashboardEntity> {
    stats
        .top_apps
        .iter()
        .take(5)
        .map(|app| DiaryDashboardEntity {
            label: app.process_name.clone(),
            description: "数字工作场所，来自前台应用活跃时长。".into(),
            active_seconds: Some(app.active_seconds),
            share: Some(app.share),
            evidence: vec![app.process_name.clone()],
            confidence: Some(0.7),
        })
        .collect()
}

fn work_type_distribution(
    stats: &DailyActivityStats,
    hourly_metrics: &[HourlyActivityMetric],
) -> Vec<DiaryDashboardEntity> {
    let mut seconds_by_type: BTreeMap<String, i64> = BTreeMap::new();
    for metric in hourly_metrics {
        if metric.active_seconds <= 0 {
            continue;
        }
        *seconds_by_type
            .entry(metric.dominant_category.as_str().to_string())
            .or_default() += metric.active_seconds;
    }
    if seconds_by_type.is_empty() {
        let total_count: usize = stats.category_mix.iter().map(|item| item.count).sum();
        for item in &stats.category_mix {
            let seconds = if total_count > 0 {
                ((stats.active_seconds as f64) * (item.count as f64 / total_count as f64)).round()
                    as i64
            } else {
                0
            };
            seconds_by_type.insert(item.activity_category.as_str().to_string(), seconds);
        }
    }
    distribution_entities(
        seconds_by_type,
        stats.active_seconds,
        "来自 hourly dominant category 的工作类型分布。",
    )
}

fn team_distribution(
    stats: &DailyActivityStats,
    five_hour_reports: &[InsightReport],
) -> Vec<DiaryDashboardTeam> {
    let mut teams: BTreeMap<String, TeamAccumulator> = BTreeMap::new();
    for report in five_hour_reports {
        let duration = (report.period_end - report.period_start)
            .num_seconds()
            .max(0);
        let names = if report.project_hints.is_empty() {
            vec!["Unknown team".to_string()]
        } else {
            report.project_hints.clone()
        };
        let seconds_per_team = if names.is_empty() {
            duration
        } else {
            duration / names.len() as i64
        };
        let primary_work_type = report
            .category_mix
            .first()
            .map(|item| item.activity_category.as_str().to_string())
            .unwrap_or_else(|| "unknown".into());
        for name in names {
            let entry = teams.entry(name).or_default();
            entry.active_seconds += seconds_per_team;
            entry.work_types.insert(primary_work_type.clone());
            if !report.summary_text.trim().is_empty() {
                entry.evidence.insert(report.summary_text.clone());
            }
        }
    }

    let total_seconds: i64 = teams
        .values()
        .map(|team| team.active_seconds)
        .sum::<i64>()
        .max(stats.active_seconds);
    let mut rows = teams
        .into_iter()
        .map(|(name, team)| {
            let work_types = team.work_types.into_iter().collect::<Vec<_>>();
            let role = work_types
                .first()
                .map(|work_type| role_label(work_type).to_string())
                .unwrap_or_else(|| "unknown role".into());
            DiaryDashboardTeam {
                name,
                active_seconds: Some(team.active_seconds),
                share: share(team.active_seconds, total_seconds),
                work_types,
                role,
                evidence: team.evidence.into_iter().take(3).collect(),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .active_seconds
            .unwrap_or_default()
            .cmp(&left.active_seconds.unwrap_or_default())
            .then_with(|| left.name.cmp(&right.name))
    });
    rows
}

#[derive(Default)]
struct TeamAccumulator {
    active_seconds: i64,
    work_types: BTreeSet<String>,
    evidence: BTreeSet<String>,
}

fn role_distribution(
    work_types: &[DiaryDashboardEntity],
    teams: &[DiaryDashboardTeam],
) -> Vec<DiaryDashboardRole> {
    let team_names = teams
        .iter()
        .map(|team| team.name.clone())
        .collect::<Vec<_>>();
    let mut roles = work_types
        .iter()
        .map(|work_type| {
            let label = role_label(&work_type.label);
            DiaryDashboardRole {
                label: label.into(),
                description: format!("由 {} 工作类型推断。", work_type.label),
                active_seconds: work_type.active_seconds,
                share: work_type.share,
                teams: team_names.clone(),
                evidence: work_type.evidence.clone(),
            }
        })
        .collect::<Vec<_>>();
    roles.sort_by(|left, right| {
        right
            .active_seconds
            .unwrap_or_default()
            .cmp(&left.active_seconds.unwrap_or_default())
            .then_with(|| left.label.cmp(&right.label))
    });
    roles
}

fn role_heterogeneity(roles: &[DiaryDashboardRole]) -> DiaryRoleHeterogeneity {
    let distinct_role_count = roles.len();
    let score = if distinct_role_count <= 1 {
        0.0
    } else {
        ((distinct_role_count as f64 - 1.0) / 4.0).min(1.0)
    };
    let level = if score < 0.34 {
        "low"
    } else if score < 0.67 {
        "medium"
    } else {
        "high"
    };
    DiaryRoleHeterogeneity {
        level: level.into(),
        score,
        summary: format!("识别到 {} 类主要角色。", distinct_role_count),
        distinct_role_count,
    }
}

fn time_blocks(
    hourly_metrics: &[HourlyActivityMetric],
    teams: &[DiaryDashboardTeam],
) -> Vec<DiaryDashboardTimeBlock> {
    let primary_team = teams.first().map(|team| team.name.clone());
    hourly_metrics
        .iter()
        .filter(|metric| metric.active_seconds > 0)
        .map(|metric| {
            let work_type = metric.dominant_category.as_str().to_string();
            let role = role_label(&work_type).to_string();
            DiaryDashboardTimeBlock {
                label: human_report_range(metric.start_at, metric.end_at),
                start_at: Some(human_report_timestamp(metric.start_at)),
                end_at: Some(human_report_timestamp(metric.end_at)),
                active_seconds: metric.active_seconds,
                primary_team: primary_team.clone(),
                primary_work_type: Some(work_type.clone()),
                role: Some(role),
                summary: format!(
                    "这一时段主要是 {}，主要窗口为 {}。",
                    work_type,
                    metric.dominant_app.as_deref().unwrap_or("Unknown")
                ),
            }
        })
        .collect()
}

fn distribution_entities(
    values: BTreeMap<String, i64>,
    total_seconds: i64,
    description: &str,
) -> Vec<DiaryDashboardEntity> {
    let mut rows = values
        .into_iter()
        .map(|(label, seconds)| DiaryDashboardEntity {
            label: label.clone(),
            description: description.into(),
            active_seconds: Some(seconds),
            share: share(seconds, total_seconds),
            evidence: vec![label],
            confidence: Some(0.65),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .active_seconds
            .unwrap_or_default()
            .cmp(&left.active_seconds.unwrap_or_default())
            .then_with(|| left.label.cmp(&right.label))
    });
    rows
}

fn role_label(activity: &str) -> &'static str {
    match activity {
        "coding" | "project_work" => "builder",
        "research" | "learning" => "researcher",
        "writing" => "writer",
        "communication" | "meeting" => "collaborator",
        "planning" => "planner",
        "admin" => "operator",
        "loafing" | "personal" | "idle" => "non-work context",
        _ => "generalist",
    }
}

fn share(seconds: i64, total_seconds: i64) -> Option<f64> {
    if total_seconds <= 0 {
        return None;
    }
    Some(seconds as f64 / total_seconds as f64)
}
