import { wcaEvents } from "cubing/puzzles";
import { randomScrambleForEvent } from "cubing/scramble";

const port = 3311;

Bun.serve({
  port,
  routes: {
    "/events": () => Response.json(Object.keys(wcaEvents)),
    "/scramble/:id": async (req) => {
      const event = req.params.id;

      if (!(event in wcaEvents)) {
        return Response.json({ error: `Event "${event}" not found` }, { status: 404 });
      }

      const scramble = await getScramble(event);
      return Response.json({ event, scramble });
    },
  },
  fetch() {
    return new Response("Not found", { status: 404 });
  },
});

async function getScramble(event: string): Promise<string> {
  const scramble = await randomScrambleForEvent(event);
  return scramble.toString();
}

console.log(`Server running on port ${port}`);
