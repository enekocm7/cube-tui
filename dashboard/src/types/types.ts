import { Temporal } from "@js-temporal/polyfill";

export const WcaEvent = {
    Cube2x2: "Cube2x2",
    Cube3x3: "Cube3x3",
    Cube4x4: "Cube4x4",
    Cube5x5: "Cube5x5",
    Cube6x6: "Cube6x6",
    Cube7x7: "Cube7x7",
    Megaminx: "Megaminx",
    Pyraminx: "Pyraminx",
    Skewb: "Skewb",
    Square1: "Square1",
    Clock: "Clock",
} as const;

export type WcaEvent = (typeof WcaEvent)[keyof typeof WcaEvent];

export const WCA_EVENT_NAMES: Record<WcaEvent, string> = {
    [WcaEvent.Cube2x2]: "2x2x2",
    [WcaEvent.Cube3x3]: "3x3x3",
    [WcaEvent.Cube4x4]: "4x4x4",
    [WcaEvent.Cube5x5]: "5x5x5",
    [WcaEvent.Cube6x6]: "6x6x6",
    [WcaEvent.Cube7x7]: "7x7x7",
    [WcaEvent.Megaminx]: "Megaminx",
    [WcaEvent.Pyraminx]: "Pyraminx",
    [WcaEvent.Skewb]: "Skewb",
    [WcaEvent.Square1]: "Square-1",
    [WcaEvent.Clock]: "Clock",
};

export const Modifier = {
    None: "None",
    PlusTwo: "PlusTwo",
    DNF: "DNF",
} as const;

export type Modifier = (typeof Modifier)[keyof typeof Modifier];

export type Time = {
    timestamp_in_millis: number;
    event: WcaEvent;
    scramble: string;
    /** Unix epoch in milliseconds — use {@link toInstant} to convert. */
    solved_at_unix_ms: number;
    modifier: Modifier;
};

export type History = {
    times: Time[];
};

/** Convert a Unix timestamp in milliseconds to a {@link Temporal.Instant}. */
export const toInstant = (unix_ms: number): Temporal.Instant => Temporal.Instant.fromEpochMilliseconds(unix_ms);
