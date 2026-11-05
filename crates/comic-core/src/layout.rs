//! The house style: the strip's size, its grid of panels, the figures' height, the type sizes and the line weights. A
//! strip is laid out in CSS px and drawn at `scale` device pixels each.

/// Every size the strip is drawn at.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    /// Device pixels to the CSS pixel.
    pub scale: f32,
    pub paper: [u8; 3],
    pub ink: [u8; 3],
    /// A strip of two or more panels, wide.
    pub strip_width: f32,
    /// A strip of one panel, wide.
    pub single_width: f32,
    pub margin: f32,
    pub gutter: f32,
    /// A panel's drawing, as tall as this much of its width.
    pub art_ratio: f32,
    /// A figure, as tall as this much of the drawing.
    pub figure_ratio: f32,
    /// The ground, this far down the drawing.
    pub ground_ratio: f32,
    pub title_cap: f32,
    pub caption_cap: f32,
    pub bubble_cap: f32,
    pub label_cap: f32,
    pub sign_cap: f32,
    pub border_width: f32,
    pub ink_width: f32,
    pub letter_width: f32,
    pub title_width: f32,
    /// A bubble's lettering, at most this much of the panel's width.
    pub bubble_width: f32,
    pub caption_pad: f32,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            scale: 2.0,
            paper: [255, 255, 255],
            ink: [0, 0, 0],
            strip_width: 1600.0,
            single_width: 800.0,
            margin: 24.0,
            gutter: 20.0,
            art_ratio: 0.82,
            figure_ratio: 0.40,
            ground_ratio: 0.93,
            title_cap: 20.0,
            caption_cap: 13.0,
            bubble_cap: 13.0,
            label_cap: 11.0,
            sign_cap: 11.0,
            border_width: 2.6,
            ink_width: 2.2,
            letter_width: 1.5,
            title_width: 2.0,
            bubble_width: 0.58,
            caption_pad: 9.0,
        }
    }
}

/// The grid for `panels` panels: columns, rows. Up to three in a row; four as two by two; five and six as two rows of
/// three.
pub fn grid(panels: usize) -> (usize, usize) {
    match panels {
        0 | 1 => (1, 1),
        2 | 3 => (panels, 1),
        4 => (2, 2),
        _ => (3, panels.div_ceil(3)),
    }
}

impl Style {
    /// The strip's width in CSS px.
    pub fn page_width(&self, panels: usize) -> f32 {
        if panels >= 2 {
            self.strip_width
        } else {
            self.single_width
        }
    }

    /// A panel's width in CSS px.
    pub fn panel_width(&self, panels: usize) -> f32 {
        let (cols, _) = grid(panels);
        ((self.page_width(panels) - 2.0 * self.margin - (cols - 1) as f32 * self.gutter)
            / cols as f32)
            .floor()
    }
}
