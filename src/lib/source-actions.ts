import { T } from '$lib/tokens';

export type SourceActionKind = 'view' | 'refresh' | 'mark-read' | 'edit' | 'remove';

export interface SourceAction {
  icon: string;
  label: string;
  kind: SourceActionKind;
  color: string;
}

export const SOURCE_ACTIONS: SourceAction[] = [
  { icon: 'list',  label: 'View feed',     kind: 'view',      color: T.ink0 },
  { icon: 'sync',  label: 'Refresh now',   kind: 'refresh',   color: T.ink0 },
  { icon: 'star',  label: 'Mark all read', kind: 'mark-read', color: T.ink0 },
  { icon: 'edit',  label: 'Edit',          kind: 'edit',      color: T.cyan },
  { icon: 'trash', label: 'Remove',        kind: 'remove',    color: T.red },
];
