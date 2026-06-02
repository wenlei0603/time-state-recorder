import type { ActivityBucket, ActivityCategory, AttentionState } from "../types";

export type ActivityBreakdownItem = {
  category: ActivityCategory;
  label: string;
  bucketCount: number;
  seconds: number;
};

export type AttentionBreakdownItem = {
  state: AttentionState;
  label: string;
  bucketCount: number;
  seconds: number;
};

export type ActivityReviewSummary = {
  bucketCount: number;
  totalBucketSeconds: number;
  dominantSeconds: number;
  totalSwitches: number;
  categoryBreakdown: ActivityBreakdownItem[];
  attentionBreakdown: AttentionBreakdownItem[];
};

const categoryLabels: Record<ActivityCategory, string> = {
  project_work: "Project work",
  research: "Research",
  writing: "Writing",
  coding: "Coding",
  communication: "Communication",
  meeting: "Meeting",
  admin: "Admin",
  learning: "Learning",
  planning: "Planning",
  loafing: "Loafing",
  personal: "Personal",
  idle: "Idle",
  unknown: "Unknown"
};

const attentionLabels: Record<AttentionState, string> = {
  deep_focus: "Deep focus",
  steady: "Steady",
  light_switching: "Light switching",
  fragmented: "Fragmented",
  away: "Away",
  unknown: "Unknown"
};

export function summarizeActivityBuckets(
  buckets: ActivityBucket[]
): ActivityReviewSummary {
  const categoryMap = new Map<ActivityCategory, ActivityBreakdownItem>();
  const attentionMap = new Map<AttentionState, AttentionBreakdownItem>();
  let totalBucketSeconds = 0;
  let dominantSeconds = 0;
  let totalSwitches = 0;

  for (const bucket of buckets) {
    totalBucketSeconds += bucket.bucketSeconds;
    dominantSeconds += bucket.dominantDurationSeconds;
    totalSwitches += bucket.switchCount;

    const categoryItem =
      categoryMap.get(bucket.activityCategory) ??
      createCategoryBreakdownItem(bucket.activityCategory);
    categoryItem.bucketCount += 1;
    categoryItem.seconds += bucket.bucketSeconds;
    categoryMap.set(bucket.activityCategory, categoryItem);

    const attentionItem =
      attentionMap.get(bucket.attentionState) ??
      createAttentionBreakdownItem(bucket.attentionState);
    attentionItem.bucketCount += 1;
    attentionItem.seconds += bucket.bucketSeconds;
    attentionMap.set(bucket.attentionState, attentionItem);
  }

  return {
    bucketCount: buckets.length,
    totalBucketSeconds,
    dominantSeconds,
    totalSwitches,
    categoryBreakdown: [...categoryMap.values()].sort(sortBreakdown),
    attentionBreakdown: [...attentionMap.values()].sort(sortBreakdown)
  };
}

export function activityCategoryLabel(category: ActivityCategory): string {
  return categoryLabels[category];
}

export function attentionStateLabel(state: AttentionState): string {
  return attentionLabels[state];
}

export function formatBucketMinutes(seconds: number): string {
  const minutes = seconds / 60;
  if (Number.isInteger(minutes)) {
    return `${minutes}m`;
  }
  return `${minutes.toFixed(1)}m`;
}

function createCategoryBreakdownItem(
  category: ActivityCategory
): ActivityBreakdownItem {
  return {
    category,
    label: activityCategoryLabel(category),
    bucketCount: 0,
    seconds: 0
  };
}

function createAttentionBreakdownItem(
  state: AttentionState
): AttentionBreakdownItem {
  return {
    state,
    label: attentionStateLabel(state),
    bucketCount: 0,
    seconds: 0
  };
}

function sortBreakdown<
  T extends { seconds: number; bucketCount: number; label: string }
>(left: T, right: T): number {
  return (
    right.seconds - left.seconds ||
    right.bucketCount - left.bucketCount ||
    left.label.localeCompare(right.label)
  );
}
