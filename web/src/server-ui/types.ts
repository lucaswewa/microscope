/**
 * Server-described UI (ADR-0024): the panels a Thing describes, which
 * ServerInterface draws. These types mirror `microscope_things::ui`. The
 * fixtures in tests/fixtures/server-ui/ are both the Rust model's
 * serialisation snapshots and the renderers' test inputs, which keeps the two
 * in step.
 */

/** A run of text, and how it's shown. */
export interface Span {
  text: string
  bold?: boolean
  italic?: boolean
  /** A link, opened outside the app. */
  href?: string
}

/** Text with a little emphasis, and links. */
export type RichText = Span[]

/** An input for a property, drawn to suit its schema. */
export interface PropertyControlElement {
  kind: 'property_control'
  thing: string
  property: string
  label: string
  /** Values to choose from, in this order, in place of free entry. */
  options?: { label: string; value: unknown }[]
  /** The step of a number's input, in place of its schema's. */
  step?: number
  /** Read the property again this many ms after writing it. */
  read_back_ms?: number
  /** `duration_hms`: a number of seconds, entered as hours, minutes and seconds. */
  input?: 'default' | 'duration_hms'
  /** Whether the Thing lacks the property. */
  broken?: boolean
}

/** One element of a panel. */
export type UiElement =
  | { kind: 'header'; text: RichText }
  | { kind: 'text'; text: RichText }
  | { kind: 'bullets'; items: RichText[] }
  | PropertyControlElement
  | { kind: 'accordion'; title: string; children: UiElement[] }
  | { kind: 'container'; layout: 'column' | 'row'; children: UiElement[] }
