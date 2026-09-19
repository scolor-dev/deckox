export type SortDirection = "asc" | "desc";

export type DataCell = string | number | boolean | null;

export type DataRow = Record<string, DataCell>;

export interface DataColumn {
  key: string;
  label: string;
  sortable?: boolean;
  align?: "start" | "end";
}

function compareCells(a: DataCell, b: DataCell): number {
  if (typeof a === "number" && typeof b === "number") return a - b;
  return String(a).localeCompare(String(b), undefined, { numeric: true, sensitivity: "base" });
}

export function sortRows(rows: DataRow[], key: string | null, direction: SortDirection): DataRow[] {
  if (key === null) return rows;
  const sign = direction === "asc" ? 1 : -1;
  return rows
    .map((row, index) => ({ row, index }))
    .sort((left, right) => {
      const a = left.row[key];
      const b = right.row[key];
      if (a === null && b === null) return left.index - right.index;
      if (a === null) return 1;
      if (b === null) return -1;
      return sign * compareCells(a, b) || left.index - right.index;
    })
    .map(({ row }) => row);
}
