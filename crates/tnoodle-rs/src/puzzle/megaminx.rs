//! The Megaminx (`MegaminxPuzzle`), scrambled with Pochmann style `R++ D--` sequences.

use std::borrow::Cow;
use std::f64::consts::PI;
use std::sync::LazyLock;

use crate::java::{self, RandomSource, deep_hash_2d, double_to_string, java_hash_order};
use crate::scrambles::{ColorScheme, Puzzle, PuzzleState, PuzzleStateAndGenerator};
use crate::svg::{Color, Dimension, Element, PathCommand, Point2D, Svg};

/// A Megaminx face, in TNoodle's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MinxFace {
    /// Up
    U,
    /// Back left
    BL,
    /// Back right
    BR,
    /// Right
    R,
    /// Front
    F,
    /// Left
    L,
    /// Down
    D,
    /// Down right
    DR,
    /// Down back right
    DBR,
    /// Back
    B,
    /// Down back left
    DBL,
    /// Down left
    DL,
}

impl MinxFace {
    /// All faces in order.
    pub const ALL: [Self; 12] = [
        Self::U,
        Self::BL,
        Self::BR,
        Self::R,
        Self::F,
        Self::L,
        Self::D,
        Self::DR,
        Self::DBR,
        Self::B,
        Self::DBL,
        Self::DL,
    ];

    /// The opposite face.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::U => Self::D,
            Self::BL => Self::DR,
            Self::BR => Self::DL,
            Self::R => Self::DBL,
            Self::F => Self::B,
            Self::L => Self::DBR,
            Self::D => Self::U,
            Self::DR => Self::BL,
            Self::DBR => Self::L,
            Self::B => Self::F,
            Self::DBL => Self::R,
            Self::DL => Self::BR,
        }
    }

    /// The face's name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::U => "U",
            Self::BL => "BL",
            Self::BR => "BR",
            Self::R => "R",
            Self::F => "F",
            Self::L => "L",
            Self::D => "D",
            Self::DR => "DR",
            Self::DBR => "DBR",
            Self::B => "B",
            Self::DBL => "DBL",
            Self::DL => "DL",
        }
    }
}

const GAP: i32 = 2;
const MINX_RAD: i32 = 30;
const CENTER_INDEX: usize = 10;

/// 12 faces of 11 stickers: 10 around the center, then the center.
type Image = [[u8; 11]; 12];

static DEFAULT_COLOR_SCHEME: [(&str, Color); 12] = [
    ("U", Color::WHITE),
    ("BL", Color::YELLOW_GOLD),
    ("BR", Color::BLUE_NAVY),
    ("R", Color::RED_VERMILION),
    ("F", Color::GREEN_DARK),
    ("L", Color::PURPLE_ORCHID),
    ("D", Color::GRAY_MEDIUM),
    ("DR", Color::YELLOW_CREAM),
    ("DBR", Color::PINK),
    ("B", Color::GREEN_LIME),
    ("DBL", Color::ORANGE_TANGERINE),
    ("DL", Color::BLUE_SKY),
];

fn swap_on_side(image: &mut Image, b: usize, f: [usize; 5], s: [usize; 5]) {
    for i in 0..3 {
        let p: Vec<(usize, usize)> = (0..5).map(|k| ((f[k] + b) % 12, (s[k] + i) % 10)).collect();
        let temp = image[p[0].0][p[0].1];
        for k in 0..4 {
            image[p[k].0][p[k].1] = image[p[k + 1].0][p[k + 1].1];
        }
        image[p[4].0][p[4].1] = temp;
    }
}

fn swap_on_face(image: &mut Image, f: usize, s: [usize; 5]) {
    let temp = image[f][s[0]];
    for k in 0..4 {
        image[f][s[k]] = image[f][s[k + 1]];
    }
    image[f][s[4]] = temp;
}

fn rotate_face(image: &mut Image, f: MinxFace) {
    swap_on_face(image, f as usize, [0, 8, 6, 4, 2]);
    swap_on_face(image, f as usize, [1, 9, 7, 5, 3]);
}

fn turn_once(image: &mut Image, face: MinxFace) {
    let s = face as usize;
    let b = if s >= 6 { 6 } else { 0 };
    match s % 6 {
        0 => swap_on_side(image, b, [1, 5, 4, 3, 2], [6, 4, 2, 0, 8]),
        1 => swap_on_side(image, b, [0, 2, 9, 10, 5], [0, 0, 6, 6, 2]),
        2 => swap_on_side(image, b, [0, 3, 8, 9, 1], [2, 2, 4, 4, 4]),
        3 => swap_on_side(image, b, [0, 4, 7, 8, 2], [4, 4, 2, 2, 6]),
        4 => swap_on_side(image, b, [0, 5, 11, 7, 3], [6, 6, 0, 0, 8]),
        _ => swap_on_side(image, b, [0, 1, 10, 11, 4], [8, 8, 8, 8, 0]),
    }
    rotate_face(image, face);
}

fn turn(image: &mut Image, face: MinxFace, dir: i32) {
    for _ in 0..dir.rem_euclid(5) {
        turn_once(image, face);
    }
}

fn swap5(image: &mut Image, p: [(usize, usize); 5]) {
    let temp = image[p[0].0][p[0].1];
    for k in 0..4 {
        image[p[k].0][p[k].1] = image[p[k + 1].0][p[k + 1].1];
    }
    image[p[4].0][p[4].1] = temp;
}

fn swap_centers(image: &mut Image, f: [usize; 5]) {
    swap5(image, f.map(|f| (f, 10)));
}

fn swap_whole_face(image: &mut Image, f: [usize; 5], s: [usize; 5]) {
    for i in 0..10 {
        swap5(image, [0, 1, 2, 3, 4].map(|k| (f[k] % 12, (s[k] + i) % 10)));
    }
    swap_centers(image, f);
}

fn big_turn_once(image: &mut Image, f: MinxFace) {
    if f == MinxFace::DBR {
        for i in 0..7 {
            swap5(
                image,
                [
                    (0, (1 + i) % 10),
                    (4, (3 + i) % 10),
                    (11, (1 + i) % 10),
                    (10, (1 + i) % 10),
                    (1, (1 + i) % 10),
                ],
            );
        }
        swap_centers(image, [0, 4, 11, 10, 1]);
        swap_whole_face(image, [2, 3, 7, 6, 9], [0, 0, 0, 8, 8]);
        rotate_face(image, MinxFace::DBR);
    } else {
        for i in 0..7 {
            swap5(
                image,
                [
                    (1, (9 + i) % 10),
                    (2, (1 + i) % 10),
                    (3, (3 + i) % 10),
                    (4, (5 + i) % 10),
                    (5, (7 + i) % 10),
                ],
            );
        }
        swap_centers(image, [1, 2, 3, 4, 5]);
        swap_whole_face(image, [11, 10, 9, 8, 7], [0, 8, 6, 4, 2]);
        rotate_face(image, MinxFace::D);
    }
}

fn big_turn(image: &mut Image, face: MinxFace, dir: i32) {
    for _ in 0..dir.rem_euclid(5) {
        big_turn_once(image, face);
    }
}

fn spin_minx(image: &mut Image, face: MinxFace, dir: i32) {
    turn(image, face, dir);
    big_turn(image, face.opposite(), 5 - dir);
}

fn spin_to_top(image: &mut Image, face: MinxFace) {
    use MinxFace::{B, BL, BR, D, DBL, DBR, DL, DR, F, L, R, U};
    match face {
        U => {}
        BL => spin_minx(image, L, 1),
        BR => {
            spin_minx(image, U, 1);
            spin_to_top(image, R);
        }
        R => {
            spin_minx(image, U, 1);
            spin_to_top(image, F);
        }
        F => spin_minx(image, L, -1),
        L => {
            spin_minx(image, U, 1);
            spin_to_top(image, BL);
        }
        D => {
            spin_minx(image, L, -2);
            spin_to_top(image, R);
        }
        DR => {
            spin_minx(image, L, -1);
            spin_to_top(image, R);
        }
        DBR => {
            spin_minx(image, U, 1);
            spin_minx(image, L, -1);
            spin_to_top(image, R);
        }
        B => {
            spin_minx(image, L, -3);
            spin_to_top(image, R);
        }
        DBL => spin_minx(image, L, 2),
        DL => spin_minx(image, L, -2),
    }
}

fn image_is_normalized(image: &Image) -> bool {
    image[MinxFace::U as usize][CENTER_INDEX] == MinxFace::U as u8
        && image[MinxFace::F as usize][CENTER_INDEX] == MinxFace::F as u8
}

fn normalize(image: &Image) -> Image {
    let mut image = *image;
    for face in MinxFace::ALL {
        if image[face as usize][CENTER_INDEX] == MinxFace::U as u8 {
            spin_to_top(&mut image, face);
            for _ in 0..5 {
                spin_minx(&mut image, MinxFace::U, 1);
                if image_is_normalized(&image) {
                    return image;
                }
            }
        }
    }
    image
}

/// The Megaminx puzzle.
#[derive(Debug, Clone, Copy, Default)]
pub struct MegaminxPuzzle;

impl MegaminxPuzzle {
    /// Creates the puzzle.
    pub const fn new() -> Self {
        Self
    }
}

/// The geometry of the drawing (the Java instance fields).
struct Geometry {
    x: f64,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    left_center_x: f64,
    left_center_y: f64,
    shift: f64,
}

static GEOMETRY: LazyLock<Geometry> = LazyLock::new(|| {
    let minx_rad = f64::from(MINX_RAD);
    let gap = f64::from(GAP);
    let x = minx_rad * (2.0 * (1.0 - java::cos(0.6 * PI))).sqrt();
    let a = minx_rad * java::cos(0.1 * PI);
    let b = x * java::cos(0.1 * PI);
    let c = x * java::cos(0.3 * PI);
    let d = x * java::sin(0.1 * PI);
    let e = x * java::sin(0.3 * PI);
    let left_center_x = gap + a + b + d / 2.0;
    let left_center_y = gap + x + minx_rad - d;
    let f = java::cos(0.1 * PI);
    let gg = java::cos(0.2 * PI);
    let magic_shift_number = d * 0.6 + minx_rad * (f + gg);
    let shift = left_center_x + magic_shift_number;
    Geometry {
        x,
        a,
        b,
        c,
        d,
        e,
        left_center_x,
        left_center_y,
        shift,
    }
});

fn unfold_height() -> f64 {
    2.0 + 3.0 * java::sin(0.3 * PI) + java::sin(0.1 * PI)
}

fn unfold_width() -> f64 {
    4.0 * java::cos(0.1 * PI) + 2.0 * java::cos(0.3 * PI)
}

fn pentagon(pointup: bool, minx_rad: i32) -> Element {
    let mut angs = [1.3, 1.7, 0.1, 0.5, 0.9];
    for a in &mut angs {
        if pointup {
            *a -= 0.2;
        }
        *a *= PI;
    }
    let r = f64::from(minx_rad);
    let x: Vec<f64> = angs.iter().map(|&a| r * java::cos(a)).collect();
    let y: Vec<f64> = angs.iter().map(|&a| r * java::sin(a)).collect();
    let mut p = Element::path();
    p.move_to(x[0], y[0]);
    for ch in 1..x.len() {
        p.line_to(x[ch], y[ch]);
    }
    // TNoodle closes the pentagon explicitly as well.
    p.line_to(x[0], y[0]);
    p.close_path();
    p
}

fn get_pentagon(x: f64, y: f64, up: bool) -> Element {
    let mut p = pentagon(up, MINX_RAD);
    p.translate_path(x, y);
    p
}

fn det(a: f64, b: f64, c: f64, d: f64) -> f64 {
    a * d - b * c
}

/// The intersection of the line through points 1 and 2 with the line through 3 and 4.
#[allow(clippy::too_many_arguments)]
pub(crate) fn line_intersection(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    x4: f64,
    y4: f64,
) -> Point2D {
    Point2D::new(
        det(det(x1, y1, x2, y2), x1 - x2, det(x3, y3, x4, y4), x3 - x4)
            / det(x1 - x2, y1 - y2, x3 - x4, y3 - y4),
        det(det(x1, y1, x2, y2), y1 - y2, det(x3, y3, x4, y4), y3 - y4)
            / det(x1 - x2, y1 - y2, x3 - x4, y3 - y4),
    )
}

/// The outline of every face of the unfolded Megaminx (`getFaceBoundaries`).
fn face_boundary(face: MinxFace) -> Element {
    let g = &*GEOMETRY;
    let gap = f64::from(GAP);
    let rad = f64::from(MINX_RAD);
    let (lcx, lcy) = (g.left_center_x, g.left_center_y);
    let s = g.shift;
    match face {
        MinxFace::U => get_pentagon(lcx, lcy, true),
        MinxFace::BL => get_pentagon(lcx - g.c, lcy - g.e, false),
        MinxFace::BR => get_pentagon(lcx + g.c, lcy - g.e, false),
        MinxFace::R => get_pentagon(lcx + g.b, lcy + g.d, false),
        MinxFace::F => get_pentagon(lcx, lcy + g.x, false),
        MinxFace::L => get_pentagon(lcx - g.b, lcy + g.d, false),
        MinxFace::D => get_pentagon(s + gap + g.a + g.b, gap + g.x + rad, false),
        MinxFace::DR => get_pentagon(s + gap + g.a + g.b - g.c, gap + g.x + g.e + rad, true),
        MinxFace::DBR => get_pentagon(s + gap + g.a, gap + g.x - g.d + rad, true),
        MinxFace::B => get_pentagon(s + gap + g.a + g.b, gap + rad, true),
        MinxFace::DBL => get_pentagon(s + gap + g.a + 2.0 * g.b, gap + g.x - g.d + rad, true),
        MinxFace::DL => get_pentagon(s + gap + g.a + g.b + g.c, gap + g.x + g.e + rad, true),
    }
}

impl Puzzle for MegaminxPuzzle {
    type State = MegaminxState;

    fn short_name(&self) -> &str {
        "minx"
    }

    fn long_name(&self) -> &str {
        "Megaminx"
    }

    fn wca_min_scramble_distance(&self) -> i32 {
        2
    }

    fn default_color_scheme_entries(&self) -> &'static [(&'static str, Color)] {
        &DEFAULT_COLOR_SCHEME
    }

    fn preferred_size(&self) -> Dimension {
        Dimension::new(
            (unfold_width() * 2.0 * f64::from(MINX_RAD) + f64::from(3 * GAP)) as i32,
            (unfold_height() * f64::from(MINX_RAD) + f64::from(2 * GAP)) as i32,
        )
    }

    fn solved_state(&self) -> MegaminxState {
        let mut image = [[0; 11]; 12];
        for (i, face) in image.iter_mut().enumerate() {
            face.fill(i as u8);
        }
        MegaminxState { image }
    }

    fn random_move_count(&self) -> i32 {
        11 * 7
    }

    fn generate_random_moves(
        &self,
        r: &mut dyn RandomSource,
    ) -> PuzzleStateAndGenerator<MegaminxState> {
        let mut scramble = String::new();
        for i in 0..7 {
            if i > 0 {
                scramble.push('\n');
            }
            let mut dir = 0;
            for j in 0..10 {
                if j > 0 {
                    scramble.push(' ');
                }
                let side = if j % 2 == 0 { 'R' } else { 'D' };
                dir = r.next_int_bounded(2);
                scramble.push(side);
                scramble.push_str(if dir == 0 { "++" } else { "--" });
            }
            scramble.push_str(" U");
            if dir != 0 {
                scramble.push('\'');
            }
        }
        let state = self
            .solved_state()
            .apply_algorithm(&scramble)
            .expect("generated megaminx scrambles are valid");
        PuzzleStateAndGenerator {
            state,
            generator: scramble,
        }
    }
}

/// The stickers of a Megaminx.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MegaminxState {
    image: Image,
}

const PRETTY_DIR: [&str; 5] = ["", "", "2", "2'", "'"];
const PRETTY_POCHMANN_DIR: [&str; 5] = ["", "+", "++", "--", "-"];
/// Pochmann moves, in the iteration order of TNoodle's `HashMap` (`R` before `D`).
const POCHMANN_FACES: [(&str, MinxFace); 2] = [("R", MinxFace::DBR), ("D", MinxFace::D)];

impl MegaminxState {
    /// The colour (as a face index) of sticker `sticker` (0..11, 10 is the center) of a face.
    pub fn sticker(&self, face: MinxFace, sticker: usize) -> u8 {
        self.image[face as usize][sticker]
    }

    fn draw_pentagon(
        svg: &mut Svg,
        p: &Element,
        state: &[u8; 11],
        rotate_ccw: usize,
        label: Option<&str>,
        scheme: &ColorScheme,
    ) {
        let mut xpoints = [0.0; 5];
        let mut ypoints = [0.0; 5];
        for (ch, cmd) in p.path_commands().iter().take(5).enumerate() {
            if let PathCommand::MoveTo(x, y) | PathCommand::LineTo(x, y) = *cmd {
                xpoints[ch] = x;
                ypoints[ch] = y;
            }
        }
        let mut xs = [0.0; 10];
        let mut ys = [0.0; 10];
        for i in 0..5 {
            xs[i] = 0.4 * xpoints[(i + 1) % 5] + 0.6 * xpoints[i];
            ys[i] = 0.4 * ypoints[(i + 1) % 5] + 0.6 * ypoints[i];
            xs[i + 5] = 0.6 * xpoints[(i + 1) % 5] + 0.4 * xpoints[i];
            ys[i + 5] = 0.6 * ypoints[(i + 1) % 5] + 0.4 * ypoints[i];
        }

        let mut ps: Vec<Element> = (0..11).map(|_| Element::path()).collect();
        let mut intpent = [Point2D::new(0.0, 0.0); 5];
        for i in 0..5 {
            intpent[i] = line_intersection(
                xs[i],
                ys[i],
                xs[5 + (3 + i) % 5],
                ys[5 + (3 + i) % 5],
                xs[(i + 1) % 5],
                ys[(i + 1) % 5],
                xs[5 + (4 + i) % 5],
                ys[5 + (4 + i) % 5],
            );
            if i == 0 {
                ps[10].move_to(intpent[i].x, intpent[i].y);
            } else {
                ps[10].line_to(intpent[i].x, intpent[i].y);
            }
        }
        ps[10].close_path();

        for i in 0..5 {
            let corner = &mut ps[2 * i];
            corner.move_to(xpoints[i], ypoints[i]);
            corner.line_to(xs[i], ys[i]);
            corner.line_to(intpent[i].x, intpent[i].y);
            corner.line_to(xs[5 + (4 + i) % 5], ys[5 + (4 + i) % 5]);
            corner.close_path();

            let edge = &mut ps[2 * i + 1];
            edge.move_to(xs[i], ys[i]);
            edge.line_to(xs[i + 5], ys[i + 5]);
            edge.line_to(intpent[(i + 1) % 5].x, intpent[(i + 1) % 5].y);
            edge.line_to(intpent[i].x, intpent[i].y);
            edge.close_path();
        }

        for (i, mut sticker) in ps.into_iter().enumerate() {
            let j = if i < 10 { (i + 2 * rotate_ccw) % 10 } else { i };
            sticker.set_stroke(Some(Color::BLACK));
            sticker.set_fill(scheme.get(MinxFace::ALL[state[j] as usize].name()).copied());
            svg.append_child(sticker);
        }

        if let Some(label) = label {
            let mut center_x = 0.0;
            let mut center_y = 0.0;
            let mut min_height = f64::MAX;
            // Java's `Double.MIN_VALUE` is the smallest positive double.
            let mut max_height = f64::from_bits(1);
            for pt in &intpent {
                center_x += pt.x;
                center_y += pt.y;
                if pt.y < min_height {
                    min_height = pt.y;
                }
                if pt.y > max_height {
                    max_height = pt.y;
                }
            }
            center_x /= intpent.len() as f64;
            center_y /= intpent.len() as f64;
            let mut text = Element::text(label, center_x, center_y);
            text.set_style("font-family", "sans-serif");
            // Horizontally center the text.
            text.set_attribute("text-anchor", "middle");
            // Vertically center it by hand (baseline adjustments are poorly supported): 20%
            // of the pentagon height centers the default 16px font.
            let pentagon_height = (max_height - min_height).abs();
            let vertical_shift = java::round(pentagon_height * 0.2) as f64;
            text.set_attribute("dy", format!("{}px", double_to_string(vertical_shift)));
            svg.append_child(text);
        }
    }
}

impl PuzzleState for MegaminxState {
    fn successors_by_name(&self) -> Vec<(String, Self)> {
        let mut successors = Vec::with_capacity(12 * 4 + 8);
        for face in MinxFace::ALL {
            for dir in 1..=4 {
                let mut image = self.image;
                turn(&mut image, face, dir);
                successors.push((
                    format!("{}{}", face.name(), PRETTY_DIR[dir as usize]),
                    Self { image },
                ));
            }
        }
        for (name, face) in POCHMANN_FACES {
            for dir in 1..5 {
                let mut image = self.image;
                big_turn(&mut image, face, dir);
                successors.push((
                    format!("{name}{}", PRETTY_POCHMANN_DIR[dir as usize]),
                    Self { image },
                ));
            }
        }
        successors
    }

    fn scramble_successors(&self) -> Vec<(String, Self)> {
        let successors = self.successors_by_name();
        let turns = ["R++", "R--", "D++", "D--", "U", "U2", "U2'", "U'"];
        java_hash_order(turns.iter().map(|&turn| {
            let state = successors
                .iter()
                .find(|(name, _)| name == turn)
                .map(|(_, s)| s.clone())
                .expect("scramble turns are megaminx moves");
            (turn.to_owned(), state)
        }))
    }

    fn java_hash_code(&self) -> i32 {
        deep_hash_2d(self.image.as_flattened(), 11)
    }

    fn solved(&self) -> Self {
        MegaminxPuzzle.solved_state()
    }

    fn normalized(&self) -> Cow<'_, Self> {
        if image_is_normalized(&self.image) {
            Cow::Borrowed(self)
        } else {
            Cow::Owned(Self {
                image: normalize(&self.image),
            })
        }
    }

    fn is_normalized(&self) -> bool {
        image_is_normalized(&self.image)
    }

    /// Draws the faces in face order. (TNoodle iterates a `HashMap` keyed by an enum, whose
    /// order depends on identity hash codes and varies between JVM runs; the elements drawn
    /// are the same.)
    fn draw(&self, scheme: &ColorScheme) -> Svg {
        let mut svg = Svg::new(MegaminxPuzzle.preferred_size());
        for face in MinxFace::ALL {
            let f = face as usize;
            let rotate_ccw = match f {
                0 => 0,
                1..=5 => 1,
                _ => 2,
            };
            let label = matches!(face, MinxFace::U | MinxFace::F).then(|| face.name());
            Self::draw_pentagon(
                &mut svg,
                &face_boundary(face),
                &self.image[f],
                rotate_ccw,
                label,
                scheme,
            );
        }
        svg
    }
}
