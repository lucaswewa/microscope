/** One invalid input in a request, from a 422 response. */
export interface ErrorIssue {
  /** Where in the input, such as `x` or `rows[2]`; empty for the input as a whole. */
  field: string
  message: string
}

/** What an error says, ready to show. */
export interface DescribedError {
  /** A heading: the problem's name, or how many inputs were invalid. */
  title?: string
  message?: string
  issues: ErrorIssue[]
  /** The error as JSON, when it has none of the shapes below. */
  json?: string
}

/**
 * Reads an error in any of the shapes the app meets. The shapes are text, an
 * `Error`, and the response bodies of a `teta-wot` server in its default
 * profile:
 * - `{"detail": "…"}`;
 * - a 422's `{"detail": [{"loc": [...], "msg": "…", "type": "…", …}]}`;
 * - a problem, `{"title", "detail", "status", "type", "instance"}`.
 */
export function describeError(error: unknown): DescribedError {
  if (typeof error === 'string') return { message: error, issues: [] }
  if (error instanceof Error) return { message: error.message, issues: [] }
  if (isRecord(error)) {
    const { detail, title } = error
    if (Array.isArray(detail)) {
      const issues = detail.filter(isRecord).map(toIssue)
      const plural = issues.length === 1 ? '' : 's'
      return { title: `${issues.length} invalid input${plural}`, issues }
    }
    if (typeof detail === 'string' || typeof title === 'string') {
      return {
        title: typeof title === 'string' ? title : undefined,
        message: typeof detail === 'string' ? detail : undefined,
        issues: [],
      }
    }
  }
  return { issues: [], json: JSON.stringify(error, null, 2) ?? String(error) }
}

function toIssue(issue: Record<string, unknown>): ErrorIssue {
  const loc: unknown[] = Array.isArray(issue.loc) ? issue.loc : []
  // The first step says which part of the request: the body, the path or the query.
  const path = ['body', 'path', 'query'].includes(loc[0] as string) ? loc.slice(1) : loc
  const field = path.reduce<string>(
    (text, step) =>
      typeof step === 'number' ? `${text}[${step}]` : text ? `${text}.${step}` : String(step),
    '',
  )
  const message = typeof issue.msg === 'string' ? issue.msg : JSON.stringify(issue)
  return { field, message }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
