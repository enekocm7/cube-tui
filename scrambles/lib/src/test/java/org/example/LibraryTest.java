package org.example;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;
import org.worldcubeassociation.tnoodle.scrambles.Puzzle;

import java.util.regex.Matcher;
import java.util.regex.Pattern;

import static org.junit.jupiter.api.Assertions.*;

class LibraryTest {

    // ─── getPuzzleFromString ────────────────────────────────────────────

    @ParameterizedTest
    @ValueSource(strings = {"222", "333", "444", "555", "666", "777",
            "minx", "pyram", "skewb", "sq1", "clock"})
    void getPuzzleFromString_returnsNonNullForValidInputs(String event) {
        Puzzle puzzle = Library.getPuzzleFromString(event);
        assertNotNull(puzzle, "Puzzle should not be null for event: " + event);
    }

    @Test
    void getPuzzleFromString_unknownInputDefaultsToThree() {
        Puzzle puzzle = Library.getPuzzleFromString("unknown");
        Puzzle three = Library.getPuzzleFromString("333");
        assertNotNull(puzzle);
        assertNotNull(three);
        assertEquals(three.getShortName(), puzzle.getShortName(),
                "Unknown input should default to 3x3");
    }

    @Test
    void getPuzzleFromString_emptyStringDefaultsToThree() {
        Puzzle puzzle = Library.getPuzzleFromString("");
        Puzzle three = Library.getPuzzleFromString("333");
        assertNotNull(puzzle);
        assertEquals(three.getShortName(), puzzle.getShortName());
    }

    // ─── generateScramble – non-null / non-empty for every event ───────

    @ParameterizedTest
    @ValueSource(strings = {"222", "333", "444", "555", "666", "777",
            "minx", "pyram", "skewb", "sq1", "clock"})
    void generateScramble_returnsNonNullNonEmpty(String event) {
        String scramble = Library.generateScramble(event);
        assertNotNull(scramble, "Scramble should not be null for: " + event);
        assertFalse(scramble.isBlank(), "Scramble should not be blank for: " + event);
    }

    // ─── 2x2x2 ─────────────────────────────────────────────────────────

    @Test
    void scramble222_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("222");
        assertNotNull(scramble);
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("[RUF]2?'?"),
                    "222 move should match [RUF]2?'?, got: " + token);
        }
    }

    @Test
    void scramble222_lengthInRange() {
        String scramble = Library.generateScramble("222");
        int moveCount = scramble.trim().split("\\s+").length;
        assertTrue(moveCount >= 4 && moveCount <= 14,
                "222 scramble should have 4-14 moves, got: " + moveCount);
    }

    // ─── 3x3x3 ─────────────────────────────────────────────────────────

    @Test
    void scramble333_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("333");
        assertNotNull(scramble);
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("[RLUDFB]2?'?"),
                    "333 move should match [RLUDFB]2?'?, got: " + token);
        }
    }

    @Test
    void scramble333_lengthInRange() {
        String scramble = Library.generateScramble("333");
        int moveCount = scramble.trim().split("\\s+").length;
        assertTrue(moveCount >= 4 && moveCount <= 25,
                "333 scramble should have 4-25 moves, got: " + moveCount);
    }

    // ─── 4x4x4 ─────────────────────────────────────────────────────────

    @Test
    void scramble444_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("444");
        assertNotNull(scramble);
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("[RLUDFBrludfb]w?2?'?"),
                    "444 move should match [RLUDFBrludfb]w?2?'?, got: " + token);
        }
    }

    @Test
    void scramble444_lengthInRange() {
        String scramble = Library.generateScramble("444");
        int moveCount = scramble.trim().split("\\s+").length;
        assertTrue(moveCount >= 30 && moveCount <= 55,
                "444 scramble should have 30-55 moves, got: " + moveCount);
    }

    // ─── 5x5x5 ─────────────────────────────────────────────────────────

    @Test
    void scramble555_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("555");
        assertNotNull(scramble);
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("[RLUDFBrludfb]w?2?'?"),
                    "555 move should match [RLUDFBrludfb]w?2?'?, got: " + token);
        }
    }

    @Test
    void scramble555_lengthInRange() {
        String scramble = Library.generateScramble("555");
        int moveCount = scramble.trim().split("\\s+").length;
        assertTrue(moveCount >= 40 && moveCount <= 75,
                "555 scramble should have 40-75 moves, got: " + moveCount);
    }

    // ─── 6x6x6 ─────────────────────────────────────────────────────────

    @Test
    void scramble666_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("666");
        assertNotNull(scramble);
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("\\d?[RLUDFBrludfb]w?\\d?'?"),
                    "666 move should match \\d?[RLUDFBrludfb]w?\\d?'?, got: " + token);
        }
    }

    @Test
    void scramble666_lengthInRange() {
        String scramble = Library.generateScramble("666");
        int moveCount = scramble.trim().split("\\s+").length;
        assertTrue(moveCount >= 50 && moveCount <= 100,
                "666 scramble should have 50-100 moves, got: " + moveCount);
    }

    // ─── 7x7x7 ─────────────────────────────────────────────────────────

    @Test
    void scramble777_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("777");
        assertNotNull(scramble);
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("\\d?[RLUDFBrludfb]w?\\d?'?"),
                    "777 move should match \\d?[RLUDFBrludfb]w?\\d?'?, got: " + token);
        }
    }

    @Test
    void scramble777_lengthInRange() {
        String scramble = Library.generateScramble("777");
        int moveCount = scramble.trim().split("\\s+").length;
        assertTrue(moveCount >= 60 && moveCount <= 120,
                "777 scramble should have 60-120 moves, got: " + moveCount);
    }

    // ─── Megaminx ──────────────────────────────────────────────────────

    @Test
    void scrambleMinx_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("minx");
        assertNotNull(scramble);
        for (String line : scramble.split("\n")) {
            String trimmed = line.trim();
            if (trimmed.isEmpty()) continue;
            assertTrue(trimmed.matches("[RD][+-]{2}(\\s+[RD][+-]{2})*\\s+U'?"),
                    "Minx line should match pattern, got: " + trimmed);
        }
    }

    @Test
    void scrambleMinx_hasSevenLines() {
        String scramble = Library.generateScramble("minx");
        assertNotNull(scramble);
        long lineCount = java.util.Arrays.stream(scramble.split("\n"))
                .map(String::trim)
                .filter(s -> !s.isEmpty())
                .count();
        assertEquals(7, lineCount, "Megaminx scramble should have 7 lines");
    }

    @Test
    void scrambleMinx_eachLineHasTenMovesPlusU() {
        String scramble = Library.generateScramble("minx");
        assertNotNull(scramble);
        for (String line : scramble.split("\n")) {
            String trimmed = line.trim();
            if (trimmed.isEmpty()) continue;
            String[] tokens = trimmed.split("\\s+");
            assertEquals(11, tokens.length,
                    "Each megaminx line should have 11 tokens (10 moves + U), got: " + tokens.length
                            + " in line: " + trimmed);
            for (int i = 0; i < 10; i++) {
                assertTrue(tokens[i].matches("[RD][+-]{2}"),
                        "Megaminx move " + i + " should be R/D with ++ or --, got: " + tokens[i]);
            }
            assertTrue(tokens[10].matches("U'?"),
                    "Last token should be U or U', got: " + tokens[10]);
        }
    }

    // ─── Pyraminx ──────────────────────────────────────────────────────

    @Test
    void scramblePyram_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("pyram");
        assertNotNull(scramble);
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("[RLUBrlub]2?'?"),
                    "Pyram move should match [RLUBrlub]2?'?, got: " + token);
        }
    }

    @Test
    void scramblePyram_lengthInRange() {
        String scramble = Library.generateScramble("pyram");
        int moveCount = scramble.trim().split("\\s+").length;
        assertTrue(moveCount >= 4 && moveCount <= 20,
                "Pyraminx scramble should have 4-20 moves, got: " + moveCount);
    }

    @Test
    void scramblePyram_containsTipMoves() {
        String scramble = Library.generateScramble("pyram");
        assertNotNull(scramble);
        boolean hasLowercase = false;
        for (String token : scramble.trim().split("\\s+")) {
            if (token.matches("[rlub]'?")) {
                hasLowercase = true;
                break;
            }
        }
        assertTrue(hasLowercase, "Pyraminx scramble should contain lowercase tip moves");
    }

    // ─── Skewb ─────────────────────────────────────────────────────────

    @Test
    void scrambleSkewb_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("skewb");
        assertNotNull(scramble);
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("[RLUB]'?"),
                    "Skewb move should match [RLUB]'?, got: " + token);
        }
    }

    @Test
    void scrambleSkewb_lengthInRange() {
        String scramble = Library.generateScramble("skewb");
        int moveCount = scramble.trim().split("\\s+").length;
        assertTrue(moveCount >= 4 && moveCount <= 20,
                "Skewb scramble should have 4-20 moves, got: " + moveCount);
    }

    // ─── Square-1 ──────────────────────────────────────────────────────

    @Test
    void scrambleSq1_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("sq1");
        assertNotNull(scramble);
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("\\(-?\\d+,-?\\d+\\)|/"),
                    "SQ-1 move should match (n,n) or /, got: " + token);
        }
    }

    @Test
    void scrambleSq1_lengthInRange() {
        String scramble = Library.generateScramble("sq1");
        int moveCount = scramble.trim().split("\\s+").length;
        assertTrue(moveCount >= 4 && moveCount <= 30,
                "Square-1 scramble should have 4-30 tokens, got: " + moveCount);
    }

    @Test
    void scrambleSq1_coordinateValuesInRange() {
        String scramble = Library.generateScramble("sq1");
        assertNotNull(scramble);
        Pattern coord = Pattern.compile("\\((-?\\d+),(-?\\d+)\\)");
        Matcher m = coord.matcher(scramble);
        while (m.find()) {
            int top = Integer.parseInt(m.group(1));
            int bottom = Integer.parseInt(m.group(2));
            assertTrue(top >= -6 && top <= 6,
                    "SQ-1 top layer value should be -6..6, got: " + top);
            assertTrue(bottom >= -6 && bottom <= 6,
                    "SQ-1 bottom layer value should be -6..6, got: " + bottom);
        }
    }

    @Test
    void scrambleSq1_hasSlashSeparators() {
        String scramble = Library.generateScramble("sq1");
        assertNotNull(scramble);
        assertTrue(scramble.contains("/"), "Square-1 scramble should contain / separators");
    }

    // ─── Clock ─────────────────────────────────────────────────────────

    @Test
    void scrambleClock_containsOnlyValidMoves() {
        String scramble = Library.generateScramble("clock");
        assertNotNull(scramble);
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("(UR|DR|DL|UL|U|R|D|L|ALL)\\d+[+-]|y2"),
                    "Clock move should match valid pattern, got: " + token);
        }
    }

    @Test
    void scrambleClock_lengthInRange() {
        String scramble = Library.generateScramble("clock");
        int moveCount = scramble.trim().split("\\s+").length;
        assertTrue(moveCount >= 10 && moveCount <= 30,
                "Clock scramble should have 10-30 tokens, got: " + moveCount);
    }

    @Test
    void scrambleClock_containsPinMoves() {
        String scramble = Library.generateScramble("clock");
        assertNotNull(scramble);
        assertTrue(scramble.contains("UR") && scramble.contains("DR")
                        && scramble.contains("DL") && scramble.contains("UL"),
                "Clock scramble should contain all pin moves (UR/DR/DL/UL)");
    }

    @Test
    void scrambleClock_containsY2Rotation() {
        String scramble = Library.generateScramble("clock");
        assertNotNull(scramble);
        assertTrue(scramble.contains("y2"), "Clock scramble should contain y2 rotation");
    }

    @Test
    void scrambleClock_moveValuesInRange() {
        String scramble = Library.generateScramble("clock");
        assertNotNull(scramble);
        Pattern movePattern = Pattern.compile("(UR|DR|DL|UL|U|R|D|L|ALL)(\\d+)([+-])");
        Matcher m = movePattern.matcher(scramble);
        while (m.find()) {
            int value = Integer.parseInt(m.group(2));
            assertTrue(value >= 0 && value <= 6,
                    "Clock move value should be 0-6, got: " + value + " in move: " + m.group());
        }
    }

    @Test
    void scrambleClock_hasTwoSections() {
        String scramble = Library.generateScramble("clock");
        assertNotNull(scramble);
        String[] parts = scramble.split("y2");
        assertEquals(2, parts.length, "Clock scramble should have 2 sections separated by y2");
        String firstSection = parts[0].trim();
        String secondSection = parts[1].trim();
        assertFalse(firstSection.isEmpty(), "First section should not be empty");
        assertFalse(secondSection.isEmpty(), "Second section should not be empty");
    }

    // ─── Default behavior (unknown event → 3x3) ────────────────────────

    @Test
    void generateScramble_unknownEventReturns333Scramble() {
        String scramble = Library.generateScramble("nonexistent");
        assertNotNull(scramble);
        assertFalse(scramble.isBlank());
        for (String token : scramble.trim().split("\\s+")) {
            assertTrue(token.matches("[RLUDFB]2?'?"),
                    "Default scramble should be 3x3 format, got: " + token);
        }
    }

    // ─── Scramble uniqueness (statistical) ─────────────────────────────

    @Test
    void generateScramble_producesDifferentScrambles() {
        java.util.Set<String> scrambles = new java.util.HashSet<>();
        for (int i = 0; i < 10; i++) {
            scrambles.add(Library.generateScramble("333"));
        }
        assertTrue(scrambles.size() > 1,
                "Generating 10 scrambles should produce at least 2 unique results");
    }

    @ParameterizedTest
    @ValueSource(strings = {"222", "333", "444", "555", "666", "777",
            "minx", "pyram", "skewb", "sq1", "clock"})
    void generateScramble_multipleCallsProduceDifferentResults(String event) {
        java.util.Set<String> scrambles = new java.util.HashSet<>();
        for (int i = 0; i < 5; i++) {
            scrambles.add(Library.generateScramble(event));
        }
        assertTrue(scrambles.size() > 1,
                event + ": generating 5 scrambles should produce at least 2 unique results");
    }
}
