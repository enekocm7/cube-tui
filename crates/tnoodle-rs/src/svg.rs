//! A dead simple SVG builder, ported from TNoodle's `svglite`.
//!
//! Rendering is byte-for-byte compatible with the Java library: attributes are emitted in
//! `java.util.HashMap` order and numbers are formatted with `Double.toString` semantics.

use std::fmt;

use crate::error::InvalidHexColorError;
use crate::java::{self, JavaHashMap, double_to_string};

/// An RGBA colour (`svglite.Color`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    /// `#ff0000`
    pub const RED: Self = Self::rgb(255, 0, 0);
    /// `#00ff00`
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    /// `#0000ff`
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    /// `#ffffff`
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// `#000000`
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// `#808080`
    pub const GRAY: Self = Self::rgb(128, 128, 128);
    /// `#ffff00`
    pub const YELLOW: Self = Self::rgb(255, 255, 0);
    /// `#ff8000`
    pub const ORANGE: Self = Self::rgb(255, 128, 0);
    /// `#7c029e`
    pub const PURPLE: Self = Self::rgb(124, 2, 158);
    /// Megaminx MF8 scheme: `#ffcc00`
    pub const YELLOW_GOLD: Self = Self::rgb(255, 204, 0);
    /// Megaminx MF8 scheme: `#0000b3`
    pub const BLUE_NAVY: Self = Self::rgb(0, 0, 179);
    /// Megaminx MF8 scheme: `#dd0000`
    pub const RED_VERMILION: Self = Self::rgb(221, 0, 0);
    /// Megaminx MF8 scheme: `#006600`
    pub const GREEN_DARK: Self = Self::rgb(0, 102, 0);
    /// Megaminx MF8 scheme: `#8a1aff`
    pub const PURPLE_ORCHID: Self = Self::rgb(138, 26, 255);
    /// Megaminx MF8 scheme: `#999999`
    pub const GRAY_MEDIUM: Self = Self::rgb(153, 153, 153);
    /// Megaminx MF8 scheme: `#ffffb3`
    pub const YELLOW_CREAM: Self = Self::rgb(255, 255, 179);
    /// Megaminx MF8 scheme: `#ff99ff`
    pub const PINK: Self = Self::rgb(255, 153, 255);
    /// Megaminx MF8 scheme: `#71e600`
    pub const GREEN_LIME: Self = Self::rgb(113, 230, 0);
    /// Megaminx MF8 scheme: `#ff8433`
    pub const ORANGE_TANGERINE: Self = Self::rgb(255, 132, 51);
    /// Megaminx MF8 scheme: `#88ddff`
    pub const BLUE_SKY: Self = Self::rgb(136, 221, 255);
    /// Clock contrast colour: `#113366`
    pub const BLUE_DEEP: Self = Self::rgb(17, 51, 102);
    /// Clock contrast colour: `#ccddee`
    pub const BLUE_BRIGHT: Self = Self::rgb(204, 221, 238);
    /// Clock contrast colour: `#88aacc`
    pub const BLUE_ICE: Self = Self::rgb(136, 170, 204);
    /// Clock contrast colour: `#446699`
    pub const BLUE_ASPHALT: Self = Self::rgb(68, 102, 153);
    /// Clock contrast colour: `#ffcc44`
    pub const YELLOW_SUNFLOWER: Self = Self::rgb(255, 204, 68);
    /// Clock contrast colour: `#cc6600`
    pub const ORANGE_BRONZE: Self = Self::rgb(204, 102, 0);

    /// An opaque colour.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    /// A colour with an alpha channel.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Unpacks an `0xAARRGGBB` integer, like `new Color(int rgba)`.
    pub const fn from_argb(argb: i32) -> Self {
        let v = argb as u32;
        Self::rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, (v >> 24) as u8)
    }

    /// Parses an HTML hex colour: `#rgb`, `rgb`, `#rrggbb` or `rrggbb`.
    ///
    /// Mirrors `new Color(String)`, including Java's `Integer.parseInt(hex, 16)` quirks: a
    /// leading sign is accepted, and the parsed value's top byte becomes the alpha channel.
    pub fn from_hex(html_hex: &str) -> Result<Self, InvalidHexColorError> {
        let hex = html_hex.strip_prefix('#').unwrap_or(html_hex);
        let chars: Vec<char> = hex.chars().collect();
        let expanded: String = match chars.len() {
            3 => chars.iter().flat_map(|&c| [c, c]).collect(),
            6 => hex.to_owned(),
            _ => return Err(InvalidHexColorError(hex.to_owned())),
        };
        // `Integer.parseInt` throws a NumberFormatException for malformed digits; it is
        // reported as an invalid colour here.
        let value = parse_java_int_radix16(&expanded)
            .ok_or_else(|| InvalidHexColorError(hex.to_owned()))?;
        Ok(Self::from_argb(value))
    }

    /// The complementary colour (`invertColor`); the result is opaque.
    #[must_use]
    pub const fn invert(self) -> Self {
        Self::rgb(255 - self.r, 255 - self.g, 255 - self.b)
    }

    /// The six lowercase hex digits of the RGB channels, without `#`.
    pub fn to_hex(self) -> String {
        format!("{:06x}", self.argb() & 0x00ff_ffff)
    }

    /// The red channel.
    pub const fn red(self) -> u8 {
        self.r
    }

    /// The green channel.
    pub const fn green(self) -> u8 {
        self.g
    }

    /// The blue channel.
    pub const fn blue(self) -> u8 {
        self.b
    }

    /// The alpha channel.
    pub const fn alpha(self) -> u8 {
        self.a
    }

    /// The packed `0xAARRGGBB` value (`getRGB`).
    pub const fn argb(self) -> i32 {
        ((self.a as u32) << 24 | (self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32)
            as i32
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<color #{}>", self.to_hex())
    }
}

/// `Integer.parseInt(s, 16)` for ASCII input: optional sign, at least one hex digit, and the
/// result must fit in an `int`.
fn parse_java_int_radix16(s: &str) -> Option<i32> {
    let (negative, digits) = match s.as_bytes().first()? {
        b'-' => (true, &s[1..]),
        b'+' => (false, &s[1..]),
        _ => (false, s),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let magnitude = i64::from_str_radix(digits, 16).ok()?;
    i32::try_from(if negative { -magnitude } else { magnitude }).ok()
}

/// A width and height in pixels (`svglite.Dimension`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dimension {
    /// The width.
    pub width: i32,
    /// The height.
    pub height: i32,
}

impl Dimension {
    /// Creates a dimension.
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<org.worldcubeassociation.tnoodle.svglite.Dimension width={} height={}>",
            self.width, self.height
        )
    }
}

/// A point with `f64` coordinates (`svglite.Point2D.Double`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2D {
    /// The x coordinate.
    pub x: f64,
    /// The y coordinate.
    pub y: f64,
}

impl Point2D {
    /// Creates a point.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A 2D affine transformation matrix (`svglite.Transform`):
///
/// ```text
/// [ a c e ]
/// [ b d f ]
/// [ 0 0 1 ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    const NEAR_THRESHOLD: f64 = 0.000_001;

    /// The identity transformation.
    pub const IDENTITY: Self = Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    /// Creates a transformation from its matrix entries.
    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// A translation by `(tx, ty)`.
    pub const fn translation(tx: f64, ty: f64) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    /// A rotation by `radians` around the origin.
    pub fn rotation(radians: f64) -> Self {
        let sin = java::sin(radians);
        let cos = java::cos(radians);
        Self::new(cos, sin, -sin, cos, 0.0, 0.0)
    }

    /// A rotation by `radians` around `(anchor_x, anchor_y)`.
    pub fn rotation_around(radians: f64, anchor_x: f64, anchor_y: f64) -> Self {
        let mut t = Self::IDENTITY;
        t.translate(-anchor_x, -anchor_y);
        t.rotate(radians);
        t.translate(anchor_x, anchor_y);
        t
    }

    /// Replaces `self` with `that × self`, i.e. applies `that` after `self`.
    pub fn concatenate(&mut self, that: &Self) {
        let (l, r) = (that, *self);
        *self = Self {
            a: l.a * r.a + l.c * r.b,
            c: l.a * r.c + l.c * r.d,
            e: l.a * r.e + l.c * r.f + l.e,
            b: l.b * r.a + l.d * r.b,
            d: l.b * r.c + l.d * r.d,
            f: l.b * r.e + l.d * r.f + l.f,
        };
    }

    /// Applies a rotation around `(anchor_x, anchor_y)` after this transformation.
    pub fn rotate_around(&mut self, radians: f64, anchor_x: f64, anchor_y: f64) {
        self.concatenate(&Self::rotation_around(radians, anchor_x, anchor_y));
    }

    /// Applies a rotation around the origin after this transformation.
    pub fn rotate(&mut self, radians: f64) {
        self.concatenate(&Self::rotation(radians));
    }

    /// Applies a translation after this transformation.
    pub fn translate(&mut self, x: f64, y: f64) {
        self.concatenate(&Self::translation(x, y));
    }

    fn is_near(a: f64, b: f64) -> bool {
        -Self::NEAR_THRESHOLD <= a - b && a - b <= Self::NEAR_THRESHOLD
    }

    /// Whether this is (within a small tolerance) the identity.
    pub fn is_identity(&self) -> bool {
        Self::is_near(self.a, 1.0)
            && Self::is_near(self.d, 1.0)
            && Self::is_near(self.c, 0.0)
            && Self::is_near(self.e, 0.0)
            && Self::is_near(self.b, 0.0)
            && Self::is_near(self.f, 0.0)
    }

    /// The SVG `matrix(a,b,c,d,e,f)` form.
    pub fn to_svg_transform(&self) -> String {
        format!(
            "matrix({},{},{},{},{},{})",
            double_to_string(self.a),
            double_to_string(self.b),
            double_to_string(self.c),
            double_to_string(self.d),
            double_to_string(self.e),
            double_to_string(self.f)
        )
    }
}

/// A segment of an SVG path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    /// `M x y`
    MoveTo(f64, f64),
    /// `L x y`
    LineTo(f64, f64),
    /// `Z`
    Close,
}

impl fmt::Display for PathCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::MoveTo(x, y) => write!(f, "M {} {}", double_to_string(x), double_to_string(y)),
            Self::LineTo(x, y) => write!(f, "L {} {}", double_to_string(x), double_to_string(y)),
            Self::Close => f.write_str("Z"),
        }
    }
}

/// An SVG element: a tag with attributes, inline style, children and an optional transform.
///
/// Path elements additionally carry their [`PathCommand`]s, which are rendered into the `d`
/// attribute.
#[derive(Debug, Clone)]
pub struct Element {
    tag: String,
    attributes: JavaHashMap<String, String>,
    style: JavaHashMap<String, String>,
    children: Vec<Element>,
    content: Option<String>,
    transform: Transform,
    path: Option<Vec<PathCommand>>,
}

impl Element {
    /// An empty element with the given tag.
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            attributes: JavaHashMap::new(),
            style: JavaHashMap::new(),
            children: Vec::new(),
            content: None,
            transform: Transform::IDENTITY,
            path: None,
        }
    }

    /// A `<g>` element.
    pub fn group() -> Self {
        Self::new("g")
    }

    /// A `<path>` element with no segments yet.
    pub fn path() -> Self {
        let mut p = Self::new("path");
        p.path = Some(Vec::new());
        p
    }

    /// A `<rect>` element.
    pub fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Self {
        let mut r = Self::new("rect");
        r.set_attribute("x", double_to_string(x));
        r.set_attribute("y", double_to_string(y));
        r.set_attribute("width", double_to_string(width));
        r.set_attribute("height", double_to_string(height));
        r
    }

    /// An `<ellipse>` element.
    pub fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> Self {
        let mut e = Self::new("ellipse");
        e.set_attribute("cx", double_to_string(cx));
        e.set_attribute("cy", double_to_string(cy));
        e.set_attribute("rx", double_to_string(rx));
        e.set_attribute("ry", double_to_string(ry));
        e
    }

    /// A circle, which svglite renders as an `<ellipse>` with equal radii.
    pub fn circle(cx: f64, cy: f64, r: f64) -> Self {
        Self::ellipse(cx, cy, r, r)
    }

    /// A `<text>` element.
    pub fn text(text: impl Into<String>, x: f64, y: f64) -> Self {
        let mut t = Self::new("text");
        t.content = Some(text.into());
        t.set_attribute("x", double_to_string(x));
        t.set_attribute("y", double_to_string(y));
        t
    }

    #[must_use]
    /// Equivalent of svglite's copy constructors (`new Path(p)`, `new Circle(c)`, ...).
    ///
    /// Like Java, the copy gets freshly sized attribute and style maps (which can change
    /// their iteration order), an identity transform, and children that are copied as plain
    /// elements (so nested paths lose their segments).
    pub fn java_copy(&self) -> Self {
        let mut copy = self.copy_as_plain_element();
        copy.path.clone_from(&self.path);
        copy
    }

    fn copy_as_plain_element(&self) -> Self {
        Self {
            tag: self.tag.clone(),
            attributes: JavaHashMap::copy_of(&self.attributes),
            style: JavaHashMap::copy_of(&self.style),
            children: self
                .children
                .iter()
                .map(Self::copy_as_plain_element)
                .collect(),
            content: self.content.clone(),
            transform: Transform::IDENTITY,
            path: None,
        }
    }

    /// The tag name.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// The text content, if any.
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// Replaces the text content.
    pub fn set_content(&mut self, content: Option<String>) {
        self.content = content;
    }

    /// The child elements.
    pub fn children(&self) -> &[Element] {
        &self.children
    }

    /// Mutable access to the child elements.
    pub fn children_mut(&mut self) -> &mut Vec<Element> {
        &mut self.children
    }

    /// Appends a child element.
    pub fn append_child(&mut self, child: Element) {
        self.children.push(child);
    }

    /// The attributes, in rendering order.
    pub fn attributes(&self) -> impl Iterator<Item = (&str, &str)> {
        self.attributes
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// The value of an attribute.
    pub fn attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(&key.to_owned()).map(String::as_str)
    }

    /// Sets an attribute.
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// The value of a style property.
    pub fn style(&self, key: &str) -> Option<&str> {
        self.style.get(&key.to_owned()).map(String::as_str)
    }

    /// Sets a style property.
    pub fn set_style(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.style.insert(key.into(), value.into());
    }

    /// The inline style string, e.g. `stroke-width:2px; stroke-linejoin:round;`.
    pub fn style_string(&self) -> String {
        self.style
            .iter()
            .map(|(k, v)| format!("{k}:{v};"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn color_to_str(c: Option<Color>) -> String {
        c.map_or_else(|| "none".to_owned(), |c| format!("#{}", c.to_hex()))
    }

    /// Sets the `fill` attribute; `None` renders as `none`.
    pub fn set_fill(&mut self, c: Option<Color>) {
        self.set_attribute("fill", Self::color_to_str(c));
    }

    /// Sets the `stroke` attribute; `None` renders as `none`.
    pub fn set_stroke(&mut self, c: Option<Color>) {
        self.set_attribute("stroke", Self::color_to_str(c));
    }

    /// Sets the stroke width, miter limit and line join style properties.
    pub fn set_stroke_style(&mut self, stroke_width: i32, miter_limit: i32, line_join: &str) {
        self.set_style("stroke-width", format!("{stroke_width}px"));
        self.set_style("stroke-miterlimit", miter_limit.to_string());
        self.set_style("stroke-linejoin", line_join);
    }

    /// Applies `t` after the current transform.
    pub fn transform(&mut self, t: &Transform) {
        self.transform.concatenate(t);
    }

    /// Replaces the transform; `None` resets it to the identity.
    pub fn set_transform(&mut self, t: Option<&Transform>) {
        self.transform = t.copied().unwrap_or(Transform::IDENTITY);
    }

    /// The current transform.
    pub fn get_transform(&self) -> Transform {
        self.transform
    }

    /// Rotates the element around an anchor point.
    pub fn rotate_around(&mut self, radians: f64, anchor_x: f64, anchor_y: f64) {
        self.transform.rotate_around(radians, anchor_x, anchor_y);
    }

    /// Rotates the element around the origin.
    pub fn rotate(&mut self, radians: f64) {
        self.transform.rotate(radians);
    }

    /// Translates the element (through its transform).
    pub fn translate(&mut self, x: f64, y: f64) {
        self.transform.translate(x, y);
    }

    /// The segments of a path element, or an empty slice for other elements.
    pub fn path_commands(&self) -> &[PathCommand] {
        self.path.as_deref().unwrap_or_default()
    }

    fn path_mut(&mut self) -> &mut Vec<PathCommand> {
        self.path.get_or_insert_with(Vec::new)
    }

    /// Starts a new sub-path at `(x, y)`.
    pub fn move_to(&mut self, x: f64, y: f64) {
        self.path_mut().push(PathCommand::MoveTo(x, y));
    }

    /// Draws a line to `(x, y)`.
    pub fn line_to(&mut self, x: f64, y: f64) {
        self.path_mut().push(PathCommand::LineTo(x, y));
    }

    /// Closes the current sub-path.
    pub fn close_path(&mut self) {
        self.path_mut().push(PathCommand::Close);
    }

    /// Moves every point of the path by `(x, y)`, unlike [`translate`](Self::translate)
    /// which changes the transform.
    pub fn translate_path(&mut self, x: f64, y: f64) {
        for c in self.path_mut() {
            if let PathCommand::MoveTo(px, py) | PathCommand::LineTo(px, py) = c {
                *px += x;
                *py += y;
            }
        }
    }

    /// The `d` attribute of a path.
    pub fn path_data(&self) -> String {
        self.path_commands()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn build_string(&self, out: &mut String, level: usize) {
        // Java's Path updates its own `d` attribute right before rendering.
        let mut path_attributes;
        let attributes = if self.path.is_some() {
            path_attributes = self.attributes.clone();
            path_attributes.insert("d".to_owned(), self.path_data());
            &path_attributes
        } else {
            &self.attributes
        };

        out.push_str(&"\t".repeat(level));
        out.push('<');
        out.push_str(&self.tag);
        for (key, value) in attributes.iter() {
            out.push(' ');
            out.push_str(key);
            out.push_str("=\"");
            out.push_str(value);
            out.push('"');
        }
        if !self.style.is_empty() {
            out.push_str(" style=\"");
            out.push_str(&self.style_string());
            out.push('"');
        }
        if !self.transform.is_identity() {
            out.push_str(" transform=\"");
            out.push_str(&self.transform.to_svg_transform());
            out.push('"');
        }
        out.push('>');
        if let Some(content) = &self.content {
            out.push_str(content);
        }
        for child in &self.children {
            out.push('\n');
            child.build_string(out, level + 1);
        }
        out.push('\n');
        out.push_str(&"\t".repeat(level));
        out.push_str("</");
        out.push_str(&self.tag);
        out.push('>');
    }
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.build_string(&mut out, 0);
        f.write_str(&out)
    }
}

/// A complete SVG document (`svglite.Svg`).
#[derive(Debug, Clone)]
pub struct Svg {
    root: Element,
}

impl Svg {
    /// An empty document of the given size.
    pub fn new(size: Dimension) -> Self {
        let mut svg = Self {
            root: Element::new("svg"),
        };
        svg.set_size(size);
        svg.root.set_attribute("version", "1.1");
        svg.root
            .set_attribute("xmlns", "http://www.w3.org/2000/svg");
        svg
    }

    /// Sets the `width`, `height` and `viewBox` attributes.
    pub fn set_size(&mut self, size: Dimension) {
        self.root
            .set_attribute("width", format!("{}px", size.width));
        self.root
            .set_attribute("height", format!("{}px", size.height));
        self.root
            .set_attribute("viewBox", format!("0 0 {} {}", size.width, size.height));
    }

    /// The size parsed back from the `width` and `height` attributes.
    pub fn size(&self) -> Dimension {
        let parse = |key: &str| {
            self.root
                .attribute(key)
                .and_then(|v| v.replace("px", "").parse().ok())
                .unwrap_or_default()
        };
        Dimension::new(parse("width"), parse("height"))
    }

    /// The root `<svg>` element.
    pub fn root(&self) -> &Element {
        &self.root
    }

    /// Mutable access to the root `<svg>` element.
    pub fn root_mut(&mut self) -> &mut Element {
        &mut self.root
    }

    /// Appends a child to the root element.
    pub fn append_child(&mut self, child: Element) {
        self.root.append_child(child);
    }

    /// The children of the root element.
    pub fn children(&self) -> &[Element] {
        self.root.children()
    }
}

impl fmt::Display for Svg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.root.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        // Java parses `RRGGBB` as `0x00RRGGBB`, so parsed colours are fully transparent.
        assert_eq!(
            Color::from_hex("#f80").unwrap(),
            Color::rgba(255, 136, 0, 0)
        );
        assert_eq!(
            Color::from_hex("00FF00").unwrap().to_hex(),
            Color::GREEN.to_hex()
        );
        assert_eq!(Color::from_hex("-00001").unwrap(), Color::from_argb(-1));
        assert!(Color::from_hex("#12345").is_err());
        assert!(Color::from_hex("zzz").is_err());
        assert!(Color::from_hex("+-0001").is_err());
        assert_eq!(Color::ORANGE.to_hex(), "ff8000");
        assert_eq!(Color::WHITE.invert(), Color::BLACK);
        assert_eq!(Color::RED.to_string(), "<color #ff0000>");
        assert_eq!(Color::from_argb(0x1234_5678).alpha(), 0x12);
        let c = Color::rgba(1, 2, 3, 4);
        assert_eq!((c.red(), c.green(), c.blue(), c.alpha()), (1, 2, 3, 4));
        assert_eq!(c.argb(), 0x0401_0203);
    }

    #[test]
    fn transforms() {
        let mut t = Transform::IDENTITY;
        assert!(t.is_identity());
        t.translate(1.0, 2.0);
        assert_eq!(t.to_svg_transform(), "matrix(1.0,0.0,0.0,1.0,1.0,2.0)");
        let r = Transform::rotation_around(std::f64::consts::PI, 1.0, 1.0);
        assert!(!r.is_identity());
        assert_eq!(Transform::default(), Transform::IDENTITY);
    }

    #[test]
    fn renders_like_svglite() {
        let mut svg = Svg::new(Dimension::new(10, 20));
        let mut p = Element::path();
        p.move_to(0.0, 0.0);
        p.line_to(1.5, 2.0);
        p.close_path();
        p.translate_path(1.0, 1.0);
        p.set_fill(Some(Color::RED));
        p.set_stroke(None);
        svg.append_child(p);
        assert_eq!(svg.size(), Dimension::new(10, 20));
        assert_eq!(
            svg.to_string(),
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 20\" width=\"10px\" version=\"1.1\" height=\"20px\">\n\
             \t<path d=\"M 1.0 1.0 L 2.5 3.0 Z\" fill=\"#ff0000\" stroke=\"none\">\n\t</path>\n</svg>"
        );
        assert_eq!(svg.children().len(), 1);
    }

    #[test]
    fn copies_drop_transform_and_child_paths() {
        let mut parent = Element::path();
        parent.move_to(1.0, 2.0);
        parent.translate(3.0, 4.0);
        parent.set_stroke_style(2, 10, "round");
        let mut child = Element::path();
        child.move_to(0.0, 0.0);
        parent.append_child(child);
        let copy = parent.java_copy();
        assert!(copy.get_transform().is_identity());
        assert_eq!(copy.path_commands(), parent.path_commands());
        assert_eq!(copy.children()[0].path_commands(), []);
        assert_eq!(copy.style("stroke-width"), Some("2px"));
        assert_eq!(Element::text("U", 1.0, 2.0).content(), Some("U"));
    }
}
