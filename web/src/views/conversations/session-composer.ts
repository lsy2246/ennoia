import type { ComposerSegment } from "./chat-types";

export type ComposerSnapshotValue = {
  body: string;
  addressedAgents: string[];
  explicitMentions?: string[];
  segments: ComposerSegment[];
};

export type ComposerPickerStateValue = {
  open: boolean;
  mode: string;
  query: string;
  selectedIndex: number;
};

export function areComposerSnapshotsEqual(
  left: ComposerSnapshotValue,
  right: ComposerSnapshotValue,
) {
  return left.body === right.body
    && stringArraysEqual(left.addressedAgents, right.addressedAgents)
    && stringArraysEqual(left.explicitMentions ?? [], right.explicitMentions ?? [])
    && composerSegmentsEqual(left.segments, right.segments);
}

export function areComposerPickerStatesEqual(
  left: ComposerPickerStateValue,
  right: ComposerPickerStateValue,
) {
  return left.open === right.open
    && left.mode === right.mode
    && left.query === right.query
    && left.selectedIndex === right.selectedIndex;
}

function stringArraysEqual(left: string[], right: string[]) {
  if (left.length !== right.length) {
    return false;
  }
  return left.every((item, index) => item === right[index]);
}

function composerSegmentsEqual(left: ComposerSegment[], right: ComposerSegment[]) {
  if (left.length !== right.length) {
    return false;
  }
  return left.every((segment, index) => composerSegmentEqual(segment, right[index]));
}

function composerSegmentEqual(left: ComposerSegment, right: ComposerSegment) {
  if (left.kind !== right.kind) {
    return false;
  }
  switch (left.kind) {
    case "text":
      return right.kind === "text" && left.value === right.value;
    case "mention":
      return right.kind === "mention"
        && left.agentId === right.agentId
        && left.label === right.label;
    case "skill":
      return right.kind === "skill"
        && left.skillId === right.skillId
        && left.label === right.label;
    case "dispatch":
      return right.kind === "dispatch"
        && left.mode === right.mode
        && left.label === right.label;
  }
}
