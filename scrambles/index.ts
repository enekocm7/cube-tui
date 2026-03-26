import { wcaEvents } from "cubing/puzzles";
import { randomScrambleForEvent } from "cubing/scramble";
import express from "express";

const app = express();
const port = 3311;

app.get("/scramble/:id", async (req, res) => {
    const event = req.params.id;

    if (!Object.keys(wcaEvents).includes(event)) {
        res.status(404).json({ error: `Event "${event}" not found` });
        return;
    }

    const scramble = await getScramble(event);
    res.json({ event, scramble });
});

app.get("/events", (_, res) => {
    res.json(Object.keys(wcaEvents));
});

async function getScramble(event: string): Promise<string> {
    const scramble = await randomScrambleForEvent(event);
    return scramble.toString();
}

app.listen(port, () => {
    console.log(`Server running on port ${port}`);
});
