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
    dashboard.work_types = normalize_work_types(dashboard.work_types);
    dashboard.teams = coalesce_teams(dashboard.teams);
    dashboard.roles = coalesce_roles(dashboard.roles, &dashboard.teams, &dashboard.work_types);
    normalize_time_blocks(&mut dashboard.time_distribution);
    dashboard.team_count = dashboard.teams.len();
    dashboard.role_heterogeneity = role_heterogeneity(&dashboard.roles);
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
        "当天记录到 {:.1} 小时活跃时间，覆盖 {} 个工作域和 {} 类贡献模式。",
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
            let domain = semantic_domain_label(&name, &report.summary_text);
            let entry = teams.entry(domain).or_default();
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
            let role = contribution_mode_label(&format!(
                "{} {} {}",
                name,
                work_types.join(" "),
                team.evidence.iter().cloned().collect::<Vec<_>>().join(" ")
            ))
            .to_string();
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
    roles: BTreeSet<String>,
    evidence: BTreeSet<String>,
}

fn role_distribution(
    work_types: &[DiaryDashboardEntity],
    teams: &[DiaryDashboardTeam],
) -> Vec<DiaryDashboardRole> {
    coalesce_roles(Vec::new(), teams, work_types)
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
        summary: format!("识别到 {} 类正交贡献模式。", distinct_role_count),
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
            let role = contribution_mode_label(&work_type).to_string();
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

fn normalize_work_types(work_types: Vec<DiaryDashboardEntity>) -> Vec<DiaryDashboardEntity> {
    let has_known = work_types.iter().any(|item| !is_unknown_label(&item.label));
    let mut by_label: BTreeMap<String, EntityAccumulator> = BTreeMap::new();
    for item in work_types {
        if has_known && is_unknown_label(&item.label) {
            continue;
        }
        let label = if is_unknown_label(&item.label) {
            "Unclassified activity".to_string()
        } else {
            item.label.trim().to_string()
        };
        let entry = by_label.entry(label.clone()).or_default();
        entry.active_seconds += item.active_seconds.unwrap_or_default();
        entry.share += item.share.unwrap_or_default();
        if entry.description.is_empty() {
            entry.description = item.description;
        }
        entry.confidence = merge_confidence(entry.confidence, item.confidence);
        for evidence in item.evidence {
            insert_limited(&mut entry.evidence, evidence, 4);
        }
    }
    let total_seconds: i64 = by_label
        .values()
        .map(|item| item.active_seconds)
        .sum::<i64>();
    let mut rows = by_label
        .into_iter()
        .map(|(label, item)| DiaryDashboardEntity {
            label,
            description: if item.description.trim().is_empty() {
                "活动方式：来自模型或 hourly category 的行为标签。".into()
            } else {
                item.description
            },
            active_seconds: optional_positive(item.active_seconds),
            share: share_or_sum(item.active_seconds, total_seconds, item.share),
            evidence: item.evidence,
            confidence: item.confidence,
        })
        .collect::<Vec<_>>();
    sort_entities(&mut rows);
    rows
}

fn coalesce_teams(teams: Vec<DiaryDashboardTeam>) -> Vec<DiaryDashboardTeam> {
    let mut by_domain: BTreeMap<String, TeamAccumulator> = BTreeMap::new();
    for team in teams {
        let context = format!(
            "{} {} {}",
            team.role,
            team.work_types.join(" "),
            team.evidence.join(" ")
        );
        let domain = semantic_domain_label(&team.name, &context);
        let entry = by_domain.entry(domain).or_default();
        entry.active_seconds += team.active_seconds.unwrap_or_default();
        for work_type in team.work_types {
            if !is_unknown_label(&work_type) {
                entry.work_types.insert(work_type);
            }
        }
        let role = contribution_mode_label(&format!("{} {}", team.role, context));
        entry.roles.insert(role.to_string());
        entry.evidence.insert(team.name);
        for evidence in team.evidence {
            entry.evidence.insert(evidence);
        }
    }

    let total_seconds = by_domain
        .values()
        .map(|team| team.active_seconds)
        .sum::<i64>();
    let mut rows = by_domain
        .into_iter()
        .map(|(name, team)| {
            let role = primary_role(&team.roles, &name, &team.work_types, &team.evidence);
            DiaryDashboardTeam {
                name,
                active_seconds: optional_positive(team.active_seconds),
                share: share(team.active_seconds, total_seconds),
                work_types: team.work_types.into_iter().collect(),
                role,
                evidence: team.evidence.into_iter().take(4).collect(),
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

fn coalesce_roles(
    roles: Vec<DiaryDashboardRole>,
    teams: &[DiaryDashboardTeam],
    work_types: &[DiaryDashboardEntity],
) -> Vec<DiaryDashboardRole> {
    let mut by_mode: BTreeMap<String, RoleAccumulator> = BTreeMap::new();
    let explicit_role_seconds = roles
        .iter()
        .filter_map(|role| role.active_seconds)
        .sum::<i64>();
    for role in roles {
        let signal = format!(
            "{} {} {} {}",
            role.label,
            role.description,
            role.teams.join(" "),
            role.evidence.join(" ")
        );
        let mode = contribution_mode_label(&signal);
        add_role_signal(
            &mut by_mode,
            mode,
            role.active_seconds,
            role.share,
            role.teams,
            role.evidence,
        );
    }
    for team in teams {
        let signal = format!(
            "{} {} {} {}",
            team.name,
            team.role,
            team.work_types.join(" "),
            team.evidence.join(" ")
        );
        let mode = contribution_mode_label(&signal);
        add_role_signal(
            &mut by_mode,
            mode,
            team.active_seconds,
            team.share,
            vec![team.name.clone()],
            team.evidence.clone(),
        );
    }
    if by_mode.is_empty() {
        for work_type in work_types {
            let mode = contribution_mode_label(&work_type.label);
            add_role_signal(
                &mut by_mode,
                mode,
                work_type.active_seconds,
                work_type.share,
                teams.iter().map(|team| team.name.clone()).collect(),
                work_type.evidence.clone(),
            );
        }
    }
    if by_mode.len() > 1 {
        by_mode.remove("Unclassified contribution");
    }
    if explicit_role_seconds > 0 {
        scale_role_seconds(&mut by_mode, explicit_role_seconds);
    }

    let total_seconds = by_mode
        .values()
        .map(|role| role.active_seconds)
        .sum::<i64>();
    let mut rows = by_mode
        .into_iter()
        .map(|(label, role)| DiaryDashboardRole {
            description: contribution_mode_description(&label).into(),
            label,
            active_seconds: optional_positive(role.active_seconds),
            share: share_or_sum(role.active_seconds, total_seconds, role.share),
            teams: role.teams,
            evidence: role.evidence,
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

fn normalize_time_blocks(blocks: &mut [DiaryDashboardTimeBlock]) {
    for block in blocks {
        if let Some(team) = block.primary_team.as_mut() {
            *team = semantic_domain_label(team, &block.summary);
        }
        if let Some(role) = block.role.as_mut() {
            *role = contribution_mode_label(&format!(
                "{} {} {}",
                role,
                block.primary_work_type.as_deref().unwrap_or_default(),
                block.summary
            ))
            .to_string();
        }
    }
}

#[derive(Default)]
struct EntityAccumulator {
    active_seconds: i64,
    share: f64,
    description: String,
    evidence: Vec<String>,
    confidence: Option<f64>,
}

#[derive(Default)]
struct RoleAccumulator {
    active_seconds: i64,
    share: f64,
    teams: Vec<String>,
    evidence: Vec<String>,
}

fn add_role_signal(
    roles: &mut BTreeMap<String, RoleAccumulator>,
    label: &str,
    active_seconds: Option<i64>,
    share: Option<f64>,
    teams: Vec<String>,
    evidence: Vec<String>,
) {
    let entry = roles.entry(label.to_string()).or_default();
    entry.active_seconds += active_seconds.unwrap_or_default();
    entry.share += share.unwrap_or_default();
    for team in teams {
        insert_limited(&mut entry.teams, team, 6);
    }
    for item in evidence {
        insert_limited(&mut entry.evidence, item, 4);
    }
}

fn scale_role_seconds(roles: &mut BTreeMap<String, RoleAccumulator>, target_total: i64) {
    let current_total = roles.values().map(|role| role.active_seconds).sum::<i64>();
    if current_total <= target_total || current_total <= 0 {
        return;
    }
    let mut allocated = 0;
    let last_key = roles.keys().last().cloned();
    for (label, role) in roles.iter_mut() {
        if Some(label) == last_key.as_ref() {
            role.active_seconds = (target_total - allocated).max(0);
        } else {
            role.active_seconds = ((role.active_seconds as f64 / current_total as f64)
                * target_total as f64)
                .round() as i64;
            allocated += role.active_seconds;
        }
    }
}

fn semantic_domain_label(name: &str, context: &str) -> String {
    let signal = normalized_signal(&format!("{name} {context}"));
    if has_any(
        &signal,
        &[
            "time state recorder",
            "tsr",
            "daily brief",
            "diary dashboard",
        ],
    ) {
        "Time State Recorder".into()
    } else if has_any(
        &signal,
        &[
            "cdp",
            "runner",
            "page_limit",
            "tls handshake",
            "9222",
            "9223",
            "9224",
            "9225",
            "browser automation",
        ],
    ) {
        "Browser automation operations".into()
    } else if has_any(
        &signal,
        &[
            "ob",
            "paper development workshop",
            "pdw",
            "notice draft",
            "research workshop",
        ],
    ) {
        "OB research and writing".into()
    } else if has_any(
        &signal,
        &[
            "paper",
            "literature",
            "论文",
            "文献",
            "实证论文",
            "案例研究",
        ],
    ) {
        "Academic research and writing".into()
    } else if has_any(&signal, &["coaching", "讲评", "教案", "题型", "question"]) {
        "Teaching and coaching".into()
    } else if has_any(&signal, &["notion", "principles os", "daily diary"]) {
        "Principles OS operations".into()
    } else if name.trim().is_empty() || is_unknown_label(name) {
        "Unclassified work".into()
    } else {
        compact_label(name)
    }
}

fn contribution_mode_label(signal: &str) -> &'static str {
    let normalized = normalized_signal(signal);
    if has_any(
        &normalized,
        &[
            "cdp",
            "runner",
            "page_limit",
            "tls handshake",
            "deploy",
            "admin",
            "operator",
            "ops",
            "运维",
            "队列",
            "盘点",
        ],
    ) {
        "Operate / maintain"
    } else if has_any(
        &normalized,
        &[
            "coding",
            "project_work",
            "builder",
            "backend",
            "implement",
            "api",
            "debug",
            "code",
            "修复",
            "开发",
            "实现",
        ],
    ) {
        "Build / implement"
    } else if has_any(
        &normalized,
        &[
            "communication",
            "meeting",
            "collaborator",
            "email",
            "mail",
            "沟通",
            "邮件",
            "协作",
        ],
    ) {
        "Coordinate / communicate"
    } else if has_any(
        &normalized,
        &[
            "research",
            "reading",
            "analysis",
            "paper",
            "literature",
            "ob",
            "研究",
            "阅读",
            "论文",
            "文献",
        ],
    ) {
        "Analyze / research"
    } else if has_any(
        &normalized,
        &[
            "writing",
            "draft",
            "report",
            "summary",
            "synthesis",
            "写作",
            "总结",
            "综述",
        ],
    ) {
        "Write / synthesize"
    } else if has_any(
        &normalized,
        &["planning", "plan", "roadmap", "决策", "规划"],
    ) {
        "Plan / prioritize"
    } else if has_any(
        &normalized,
        &["learning", "course", "coaching", "teach", "学习", "教学"],
    ) {
        "Learn / coach"
    } else if has_any(&normalized, &["loafing", "personal", "idle", "away"]) {
        "Personal / away"
    } else {
        "Unclassified contribution"
    }
}

fn contribution_mode_description(label: &str) -> &'static str {
    match label {
        "Build / implement" => "贡献模式：实现、调试、开发或技术构建。",
        "Analyze / research" => "贡献模式：阅读、分析、研究或证据整理。",
        "Write / synthesize" => "贡献模式：写作、总结、整合或文档成稿。",
        "Coordinate / communicate" => "贡献模式：邮件、会议、协作沟通或对齐。",
        "Plan / prioritize" => "贡献模式：计划、排程、取舍或优先级判断。",
        "Operate / maintain" => "贡献模式：部署、排障、队列、额度或运行维护。",
        "Learn / coach" => "贡献模式：学习、讲解、教学准备或辅导。",
        "Personal / away" => "贡献模式：非工作上下文或离开状态。",
        _ => "贡献模式：证据不足，暂不细分。",
    }
}

fn primary_role(
    roles: &BTreeSet<String>,
    name: &str,
    work_types: &BTreeSet<String>,
    evidence: &BTreeSet<String>,
) -> String {
    let signal = format!(
        "{} {} {}",
        name,
        work_types.iter().cloned().collect::<Vec<_>>().join(" "),
        evidence.iter().cloned().collect::<Vec<_>>().join(" ")
    );
    let inferred = contribution_mode_label(&signal);
    if inferred != "Unclassified contribution" {
        return inferred.into();
    }
    roles
        .iter()
        .find(|role| role.as_str() != "Unclassified contribution")
        .cloned()
        .unwrap_or_else(|| "Unclassified contribution".into())
}

fn normalized_signal(value: &str) -> String {
    value.trim().to_lowercase()
}

fn has_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn is_unknown_label(value: &str) -> bool {
    let normalized = value.trim().to_lowercase();
    normalized.is_empty()
        || matches!(
            normalized.as_str(),
            "unknown" | "unknown team" | "unknown role" | "generalist" | "unclassified"
        )
}

fn compact_label(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= 64 {
        return trimmed.to_string();
    }
    let mut compact = trimmed.chars().take(61).collect::<String>();
    compact.push_str("...");
    compact
}

fn insert_limited(values: &mut Vec<String>, value: String, limit: usize) {
    let trimmed = value.trim();
    if trimmed.is_empty() || values.iter().any(|item| item == trimmed) {
        return;
    }
    if values.len() < limit {
        values.push(trimmed.to_string());
    }
}

fn optional_positive(value: i64) -> Option<i64> {
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

fn share_or_sum(seconds: i64, total_seconds: i64, share_sum: f64) -> Option<f64> {
    share(seconds, total_seconds).or_else(|| {
        if share_sum > 0.0 {
            Some(share_sum.min(1.0))
        } else {
            None
        }
    })
}

fn merge_confidence(current: Option<f64>, next: Option<f64>) -> Option<f64> {
    match (current, next) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn sort_entities(rows: &mut [DiaryDashboardEntity]) {
    rows.sort_by(|left, right| {
        right
            .active_seconds
            .unwrap_or_default()
            .cmp(&left.active_seconds.unwrap_or_default())
            .then_with(|| left.label.cmp(&right.label))
    });
}

fn share(seconds: i64, total_seconds: i64) -> Option<f64> {
    if total_seconds <= 0 {
        return None;
    }
    Some(seconds as f64 / total_seconds as f64)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use chrono::{DateTime, Utc};

    use super::*;
    use crate::models::{
        ActivityCategory, ActivityCategoryCount, DailyAppActivity, DiaryDashboard,
        DiaryDashboardRole, DiaryDashboardTeam,
    };

    #[test]
    fn fallback_groups_fragmented_project_hints_into_orthogonal_work_domains() {
        let stats = sample_stats(10_800);
        let reports = vec![
            report(
                "2026-06-11T01:00:00Z",
                "2026-06-11T02:00:00Z",
                ActivityCategory::Communication,
                vec![
                    "Individual level OB research workshop 6月18日下午",
                    "OB notice draft邮件",
                    "OB term Paper Development Workshop (PDW) 6月17日",
                    "期末论文格式（文献综述/案例研究/实证论文）",
                ],
                "围绕 OB workshop、notice 邮件和期末论文格式做沟通与研究写作准备。",
            ),
            report(
                "2026-06-11T02:00:00Z",
                "2026-06-11T03:00:00Z",
                ActivityCategory::Coding,
                vec!["Time State Recorder"],
                "修复 Time State Recorder dashboard 语义聚合。",
            ),
            report(
                "2026-06-11T03:00:00Z",
                "2026-06-11T04:00:00Z",
                ActivityCategory::Admin,
                vec!["9222/9223/9224/9225四个CDP runner的page_limit额度盘点与串行队列定义"],
                "盘点 CDP runner page_limit、队列和 TLS handshake eof 失败。",
            ),
        ];

        let dashboard = build_fallback_dashboard(None, &stats, &[], &reports);
        let team_names = dashboard
            .teams
            .iter()
            .map(|team| team.name.as_str())
            .collect::<Vec<_>>();

        assert_domains(
            &team_names,
            &[
                "OB research and writing",
                "Time State Recorder",
                "Browser automation operations",
            ],
        );
        assert_eq!(dashboard.team_count, 3);
        assert!(dashboard.teams.iter().all(|team| {
            !team.name.contains("workshop")
                && !team.name.contains("notice")
                && !team.name.contains("page_limit")
        }));

        let role_labels = dashboard
            .roles
            .iter()
            .map(|role| role.label.as_str())
            .collect::<Vec<_>>();
        assert!(role_labels.contains(&"Coordinate / communicate"));
        assert!(role_labels.contains(&"Build / implement"));
        assert!(role_labels.contains(&"Operate / maintain"));
        assert!(role_labels
            .iter()
            .all(|label| !matches!(*label, "builder" | "generalist" | "collaborator")));
    }

    #[test]
    fn normalize_model_dashboard_coalesces_fragments_and_relabels_modes() {
        let dashboard = DiaryDashboard {
            overview: "当天记录到 2.7 小时活跃时间，覆盖 23 个团队线索和 2 类角色。".into(),
            roles: vec![
                DiaryDashboardRole {
                    label: "builder".into(),
                    description: "由 coding 工作类型推断。".into(),
                    active_seconds: Some(7_560),
                    share: Some(0.8),
                    teams: vec!["Time State Recorder".into()],
                    evidence: vec!["coding".into()],
                },
                DiaryDashboardRole {
                    label: "generalist".into(),
                    description: "由 unknown 工作类型推断。".into(),
                    active_seconds: Some(1_980),
                    share: Some(0.2),
                    teams: vec![],
                    evidence: vec!["unknown".into()],
                },
            ],
            team_count: 23,
            teams: vec![
                team(
                    "Individual level OB research workshop 6月18日下午",
                    "collaborator",
                    "communication",
                ),
                team("OB notice draft邮件", "collaborator", "communication"),
                team(
                    "OB term Paper Development Workshop (PDW) 6月17日",
                    "collaborator",
                    "communication",
                ),
                team(
                    "期末论文格式（文献综述/案例研究/实证论文）",
                    "collaborator",
                    "communication",
                ),
                team("Time State Recorder", "builder", "coding"),
                team(
                    "9222/9223/9224/9225四个CDP runner的page_limit额度盘点与串行队列定义(9222 tls handshake eof断流)",
                    "builder",
                    "coding",
                ),
            ],
            ..DiaryDashboard::default()
        };

        let normalized = normalize_dashboard(dashboard);
        let team_names = normalized
            .teams
            .iter()
            .map(|team| team.name.as_str())
            .collect::<Vec<_>>();

        assert_domains(
            &team_names,
            &[
                "Academic research and writing",
                "OB research and writing",
                "Time State Recorder",
                "Browser automation operations",
            ],
        );
        assert_eq!(normalized.team_count, 4);
        assert_eq!(
            normalized.role_heterogeneity.summary,
            "识别到 3 类正交贡献模式。"
        );
        assert!(normalized
            .roles
            .iter()
            .any(|role| role.label == "Build / implement"));
        assert!(normalized
            .roles
            .iter()
            .any(|role| role.label == "Coordinate / communicate"));
        assert_eq!(
            normalized
                .roles
                .iter()
                .map(|role| role.active_seconds.unwrap_or_default())
                .sum::<i64>(),
            9_540
        );
        assert!(normalized
            .roles
            .iter()
            .all(|role| !matches!(role.label.as_str(), "builder" | "generalist")));
    }

    #[test]
    fn semantic_domain_keeps_generic_paper_work_out_of_ob() {
        assert_eq!(
            semantic_domain_label("期末论文格式（文献综述/案例研究/实证论文）", ""),
            "Academic research and writing"
        );
        assert_eq!(
            semantic_domain_label(
                "期末论文格式（文献综述/案例研究/实证论文）",
                "围绕 OB workshop、notice 邮件和期末论文格式做沟通与研究写作准备。"
            ),
            "OB research and writing"
        );
    }

    fn sample_stats(active_seconds: i64) -> DailyActivityStats {
        DailyActivityStats {
            date: "2026-06-11".into(),
            period_start: ts("2026-06-10T16:00:00Z"),
            period_end: ts("2026-06-11T16:00:00Z"),
            active_seconds,
            active_hours: active_seconds as f64 / 3600.0,
            window_event_count: 10,
            switch_count: 3,
            distinct_app_count: 2,
            top_apps: vec![DailyAppActivity {
                process_name: "Code.exe".into(),
                active_seconds,
                share: 1.0,
            }],
            category_mix: vec![
                ActivityCategoryCount {
                    activity_category: ActivityCategory::Communication,
                    count: 1,
                },
                ActivityCategoryCount {
                    activity_category: ActivityCategory::Coding,
                    count: 1,
                },
                ActivityCategoryCount {
                    activity_category: ActivityCategory::Admin,
                    count: 1,
                },
            ],
            input_chars: 0,
            input_events: 0,
            screenshot_count: 0,
            high_res_screenshot_count: 0,
            visual_window_count: 0,
            five_hour_report_count: 3,
            first_activity_at: None,
            last_activity_at: None,
        }
    }

    fn assert_domains(actual: &[&str], expected: &[&str]) {
        let expected = expected.iter().copied().collect::<BTreeSet<_>>();
        let actual = actual.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);
    }

    fn report(
        start: &str,
        end: &str,
        category: ActivityCategory,
        hints: Vec<&str>,
        summary: &str,
    ) -> InsightReport {
        InsightReport {
            id: 0,
            period_start: ts(start),
            period_end: ts(end),
            generated_at: ts(end),
            report_kind: "5h".into(),
            model_provider: "test".into(),
            model_name: "test".into(),
            summary_text: summary.into(),
            category_mix: vec![ActivityCategoryCount {
                activity_category: category,
                count: 1,
            }],
            project_hints: hints.into_iter().map(str::to_string).collect(),
            evidence_count: 1,
            error: None,
        }
    }

    fn team(name: &str, role: &str, work_type: &str) -> DiaryDashboardTeam {
        DiaryDashboardTeam {
            name: name.into(),
            active_seconds: Some(3_600),
            share: Some(0.1),
            work_types: vec![work_type.into()],
            role: role.into(),
            evidence: vec![name.into()],
        }
    }

    fn ts(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }
}
