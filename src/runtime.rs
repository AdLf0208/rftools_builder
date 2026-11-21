// RFTools Builder for Horizon — the shape geometry and the Builder's scan (pure).
//
// Ported from RFToolsBuilder 1.21-7.0.8 (`McJtyMods/RFToolsBuilder`, MIT),
// Minecraft Java 1.21.1 NeoForge, branch `1.21_neo` at commit
// `4090494663bd15e5776d9e407a59ec982129ef1f` (`gradle.properties`
// `version=1.21-7.0.8`, the jar's own version). The upstream source was read for
// behaviour only; this is a clean-room Rust rewrite.
//
// Slice 1 is the Builder's **shaped** mode: a shape card names a shape, a size,
// an offset and whether the shape is solid or hollow, and the Builder places a
// block into every replaceable cell of that shape. The quarry, the movers, the
// shields, the space chamber and the composer are not carried (notes.md).
//
// Everything here is pure: no `mod_api` type, no host, no world. The glue in
// `support.rs` reads the game, calls these rules and writes the result back.

/// The block this slice implements, exactly as the content declares it.
pub const BUILDER: &str = "rftoolsbuilder:builder";
/// The one shape card this slice carries (`rftoolsbuilder:shape_card_def`).
pub const SHAPE_CARD: &str = "rftoolsbuilder:shape_card_def";
/// The item component that carries a card's shape (`component_types`).
pub const COMPONENT: &str = "rftoolsbuilder:shape_card";

/// `BuilderConfiguration.maxBuilderDimension`: the largest side a card's
/// dimension is clamped to before the shape is built.
pub const MAX_DIMENSION: i32 = 512;
/// `BuilderConfiguration.maxBuilderOffset`: the farthest a card's offset is
/// clamped to, either way.
pub const MAX_OFFSET: i32 = 260;
/// `ShapeCardItem.MAXIMUM_COUNT`: a card whose shape holds more cells than this
/// is refused (the GUI shows `#Blocks: ++` past it).
pub const MAX_CARD_CELLS: u64 = 50_000_000;
/// `maxSpaceChamberDimension^3` (128³): the most cells one Builder pass keeps,
/// the cap upstream passes to `composeFormula` from `getCachedBlocks`.
pub const MAX_PASS_CELLS: u64 = 2_097_152;
/// `BuilderConfiguration.quarryBaseSpeed`: blocks the Builder handles in one
/// tick at infusion 0 (the shaped path's `checkStateServerShaped`).
pub const BLOCKS_PER_TICK: u32 = 8;
/// The most shape cells one `/rftoolsbuilder status` readout may inspect. The card limit allows
/// 50,000,000, and a sparse shape (a shell, a torus) has to look at each of them to count its own:
/// enumerating that on the server thread stall the tick (review 1276-rftoolsbuilder). A status
/// answer therefore stops after this many candidates and says the count is at least what it found.
/// A box answers from its dimensions and never enumerates.
pub const STATUS_CANDIDATES: u64 = 250_000;

/// `100 * quarryBaseSpeed`: the most cells one tick looks at. Upstream's
/// `handleBlockShaped` looks at up to 100 cells per call and is called
/// `quarryBaseSpeed` times a tick, so a scan never touches more than this.
pub const LOOKUPS_PER_TICK: u32 = 800;

/// `ShapeCardData.DEFAULT_DIMENSIONS`: a fresh card is a 5×5×5 box.
pub const DEFAULT_DIMENSION: [i32; 3] = [5, 5, 5];

/// The shapes a card can name (`mcjty.rftoolsbuilder.shapes.Shape` minus the
/// three this slice does not carry: composition, scan, and the space card).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    /// `SHAPE_BOX`: the card's dimension, as a box.
    Box,
    /// `SHAPE_TOPDOME`: the sphere's upper half.
    TopDome,
    /// `SHAPE_BOTTOMDOME`: the sphere's lower half.
    BottomDome,
    /// `SHAPE_SPHERE`.
    Sphere,
    /// `SHAPE_CYLINDER`: the ellipse over the whole height.
    Cylinder,
    /// `SHAPE_CAPPEDCYLINDER`: the ellipse clipped to the card's height.
    CappedCylinder,
    /// `SHAPE_PRISM`: a box narrowing towards the top.
    Prism,
    /// `SHAPE_TORUS`.
    Torus,
    /// `SHAPE_HEART`.
    Heart,
    /// `SHAPE_CONE`.
    Cone,
}

/// Which way a composition child is turned (`ShapeRotation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Rotation {
    /// `NONE`.
    #[default]
    None,
    /// `X`: y and z swap.
    X,
    /// `Y`: x and z swap.
    Y,
    /// `Z`: x and y swap.
    Z,
}

/// A card's turn and mirror (`ShapeModifier`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifier {
    /// Which way the shape is turned.
    pub rotation: Rotation,
    /// Whether the shape is mirrored top to bottom inside its box.
    pub flip_y: bool,
}

/// One shape card's parameters: the subset of upstream's `ShapeCardData` that
/// decides where a shape is. `channel`, `tagMatching`, `voiding`, `scanId`,
/// `children`, `ghostBlock` and the corner-selection `mode` are not carried
/// (notes.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Card {
    /// The shape this card draws.
    pub shape: Shape,
    /// Solid (every cell) or hollow (the shape's shell).
    pub solid: bool,
    /// The box the shape is fitted to, as the card stores it.
    pub dimension: [i32; 3],
    /// Where the shape's centre sits relative to the Builder.
    pub offset: [i32; 3],
    /// The card's turn and mirror.
    pub modifier: Modifier,
}

/// Why a card cannot be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardError {
    /// `ShapeCardItem.MAXIMUM_COUNT`: the shape holds more cells than a card
    /// may name.
    TooLarge {
        /// The cells the clamped dimension holds.
        cells: u64,
        /// `MAX_CARD_CELLS`.
        limit: u64,
    },
}

/// Why the Builder stopped at a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalReason {
    /// The cell is empty and nothing the Builder can place is in reach.
    NoMaterial,
    /// The shape lies outside the world's build height.
    OutOfWorld,
    /// The cell is protected: outside the world border, or spawn-protected.
    Protected,
}

/// The cell the Builder is waiting at, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// Where the Builder stopped.
    pub at: [i32; 3],
    /// What stopped it.
    pub reason: RefusalReason,
}

/// How far one scan has got.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// Cells a block was placed into.
    pub placed: u64,
    /// Cells looked at that needed nothing (already filled, or not in the shape).
    pub skipped: u64,
}

/// A Builder's scan: where it looks next, and the box it walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scan {
    /// The card being built.
    pub card: Card,
    /// The Builder's own position.
    pub origin: [i32; 3],
    /// The low corner of the walk, from the (turned) dimension and the offset.
    pub min: [i32; 3],
    /// The high corner, exclusive.
    pub max: [i32; 3],
    /// The next cell the walk looks at; `None` before the first tick and after
    /// a completed pass.
    pub cursor: Option<[i32; 3]>,
}

/// What a status readout says about one Builder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    /// The Builder's position.
    pub at: [i32; 3],
    /// Its card, when one is installed and usable.
    pub card: Option<Card>,
    /// Why the card is unusable, when it is installed but refused.
    pub refused_card: Option<CardError>,
    /// How far the scan has got.
    pub progress: Progress,
    /// Where it stopped, when it did.
    pub refusal: Option<Refusal>,
    /// Whether the pass has finished.
    pub complete: bool,
    /// The cells the readout prints: the pass's cached count when the block entity holds one, so a
    /// repeated `/rftoolsbuilder status` costs nothing. `None` computes it within
    /// [`STATUS_CANDIDATES`] candidates.
    pub cells: Option<CellCount>,
}

/// A shape's cell count as a readout prints it: the number found, and whether the shape holds
/// more (past [`MAX_PASS_CELLS`] kept cells, or past the candidates the readout may inspect).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellCount {
    /// The cells counted, at least.
    pub cells: u64,
    /// Whether the shape holds more than `cells`.
    pub more: bool,
}

impl Default for Card {
    /// `ShapeCardData.DEFAULT`: a solid 5×5×5 box around the Builder.
    fn default() -> Self {
        Self {
            shape: Shape::Box,
            solid: true,
            dimension: DEFAULT_DIMENSION,
            offset: [0, 0, 0],
            modifier: Modifier::default(),
        }
    }
}

impl Shape {
    /// Every shape this slice carries, in the jar's enum order.
    pub const ALL: &'static [Shape] = &[
        Shape::Box,
        Shape::TopDome,
        Shape::BottomDome,
        Shape::Sphere,
        Shape::Cylinder,
        Shape::CappedCylinder,
        Shape::Prism,
        Shape::Torus,
        Shape::Heart,
        Shape::Cone,
    ];

    /// The jar's enum name, as `Shape.getSerializedName` writes it.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Shape::Box => "SHAPE_BOX",
            Shape::TopDome => "SHAPE_TOPDOME",
            Shape::BottomDome => "SHAPE_BOTTOMDOME",
            Shape::Sphere => "SHAPE_SPHERE",
            Shape::Cylinder => "SHAPE_CYLINDER",
            Shape::CappedCylinder => "SHAPE_CAPPEDCYLINDER",
            Shape::Prism => "SHAPE_PRISM",
            Shape::Torus => "SHAPE_TORUS",
            Shape::Heart => "SHAPE_HEART",
            Shape::Cone => "SHAPE_CONE",
        }
    }

    /// The name the card's tooltip shows (`Shape.getDescription`).
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Shape::Box => "Box",
            Shape::TopDome => "Top Dome",
            Shape::BottomDome => "Bottom Dome",
            Shape::Sphere => "Sphere",
            Shape::Cylinder => "Cylinder",
            Shape::CappedCylinder => "Capped Cylinder",
            Shape::Prism => "Prism",
            Shape::Torus => "Torus",
            Shape::Heart => "Heart",
            Shape::Cone => "Cone",
        }
    }

    /// The shape a name, enum id or description names, ignoring case, spaces
    /// and underscores (`box`, `SHAPE_BOX`, `Capped Cylinder`, `cappedcylinder`).
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let wanted: String = name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        Self::ALL.iter().copied().find(|shape| {
            let id: String = shape
                .id()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
                .collect();
            let description: String = shape
                .description()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
                .collect();
            wanted == id || wanted == description
        })
    }
}

impl Rotation {
    /// The jar's `ShapeRotation.getCode`.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Rotation::None => "0",
            Rotation::X => "X",
            Rotation::Y => "Y",
            Rotation::Z => "Z",
        }
    }

    /// The rotation a code names (`ShapeRotation.getByName`); `0` is none.
    #[must_use]
    pub fn parse(code: &str) -> Option<Self> {
        match code.trim().to_ascii_uppercase().as_str() {
            "0" | "NONE" => Some(Rotation::None),
            "X" => Some(Rotation::X),
            "Y" => Some(Rotation::Y),
            "Z" => Some(Rotation::Z),
            _ => None,
        }
    }

    /// The box a shape of this dimension is turned into
    /// (`ShapeRotation.transformDimension`).
    #[must_use]
    pub fn transform_dimension(self, [x, y, z]: [i32; 3]) -> [i32; 3] {
        match self {
            Rotation::None => [x, y, z],
            Rotation::X => [x, z, y],
            Rotation::Y => [z, y, x],
            Rotation::Z => [y, x, z],
        }
    }
}

impl Card {
    /// This card with the jar's clamps applied: the dimension into
    /// `0..=MAX_DIMENSION`, the offset into `-MAX_OFFSET..=MAX_OFFSET`
    /// (`getClampedDimension` / `getClampedOffset`).
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            dimension: self.dimension.map(|d| d.clamp(0, MAX_DIMENSION)),
            offset: self.offset.map(|o| o.clamp(-MAX_OFFSET, MAX_OFFSET)),
            ..self
        }
    }

    /// The cells the card's *requested* dimension names, before clamping: the
    /// product, saturating.
    #[must_use]
    pub fn requested_cells(self) -> u64 {
        cells_of(self.dimension.map(|d| d.max(0)))
    }

    /// The cells the clamped bounding box holds: the product of the clamped
    /// dimension, saturating.
    #[must_use]
    pub fn box_cells(self) -> u64 {
        cells_of(self.clamped().dimension)
    }

    /// This card ready to build, or `CardError::TooLarge` when its clamped box
    /// holds more than [`MAX_CARD_CELLS`] (upstream's `#Blocks: ++` refusal).
    ///
    /// # Errors
    ///
    /// `CardError::TooLarge` carries the clamped box's cell count and the limit.
    pub fn validate(self) -> Result<Self, CardError> {
        let card = self.clamped();
        let cells = card.box_cells();
        if cells > MAX_CARD_CELLS {
            Err(CardError::TooLarge {
                cells,
                limit: MAX_CARD_CELLS,
            })
        } else {
            Ok(card)
        }
    }

    /// `Solid` or `Hollow`, as the card's tooltip says it.
    #[must_use]
    pub fn solid_word(self) -> &'static str {
        if self.solid { "Solid" } else { "Hollow" }
    }

    /// The shape's low corner for a Builder at `origin`
    /// (`ShapeCardItem.getMinCorner`, on the turned dimension).
    #[must_use]
    pub fn min_corner(self, origin: [i32; 3]) -> [i32; 3] {
        let dim = self.modifier.rotation.transform_dimension(self.dimension);
        let mut out = [0; 3];
        for axis in 0..3 {
            out[axis] = origin[axis] - dim[axis] / 2 + self.offset[axis];
        }
        out
    }

    /// The high corner, exclusive (`getMaxCorner`).
    #[must_use]
    pub fn max_corner(self, origin: [i32; 3]) -> [i32; 3] {
        let dim = self.modifier.rotation.transform_dimension(self.dimension);
        let min = self.min_corner(origin);
        [min[0] + dim[0], min[1] + dim[1], min[2] + dim[2]]
    }

    /// Whether `pos` is a cell of this shape for a Builder at `origin`: inside
    /// the walked box **and** inside the shape (the jar's `composeFormula`
    /// iterates the box and asks the formula).
    #[must_use]
    pub fn contains(self, origin: [i32; 3], pos: [i32; 3]) -> bool {
        let min = self.min_corner(origin);
        let max = self.max_corner(origin);
        if (0..3).any(|axis| pos[axis] < min[axis] || pos[axis] >= max[axis]) {
            return false;
        }
        if self.modifier == Modifier::default() {
            return self.inside(origin, pos);
        }
        // The jar's composition transform (`FormulaComposition.isInside`): turn
        // the relative coordinate about the card's offset, mirror it inside the
        // walked box, then ask the shape's own formula.
        let rel = [pos[0] - origin[0], pos[1] - origin[1], pos[2] - origin[2]];
        let [ox, oy, oz] = self.offset;
        let (mut tx, mut ty, mut tz) = (rel[0], rel[1], rel[2]);
        match self.modifier.rotation {
            Rotation::None => {}
            Rotation::X => {
                ty = (rel[2] - oz) + oy;
                tz = (rel[1] - oy) + oz;
            }
            Rotation::Y => {
                tx = (rel[2] - oz) + ox;
                tz = (rel[0] - ox) + oz;
            }
            Rotation::Z => {
                tx = (rel[1] - oy) + ox;
                ty = (rel[0] - ox) + oy;
            }
        }
        if self.modifier.flip_y {
            let p1 = min[1] - origin[1];
            let p2 = max[1] - origin[1];
            ty = p1 + p2 - 1 - ty;
        }
        self.inside(origin, [origin[0] + tx, origin[1] + ty, origin[2] + tz])
    }

    /// The shape's own test, with the box left out: the jar's formula, which
    /// for the cylinder ignores the height (so a hollow one-block-high
    /// cylinder has no shell — upstream's own behaviour, kept).
    fn inside(self, origin: [i32; 3], pos: [i32; 3]) -> bool {
        if self.solid {
            return inside_shape(self.shape, self.dimension, self.offset, origin, pos);
        }
        // Hollow: the jar's `correctFormula(false)` replaces the test with the
        // border test, which asks the *solid* test of the six neighbours.
        const NEIGHBOURS: [[i32; 3]; 6] = [
            [-1, 0, 0],
            [1, 0, 0],
            [0, -1, 0],
            [0, 1, 0],
            [0, 0, -1],
            [0, 0, 1],
        ];
        inside_shape(self.shape, self.dimension, self.offset, origin, pos)
            && NEIGHBOURS.iter().any(|step| {
                !inside_shape(
                    self.shape,
                    self.dimension,
                    self.offset,
                    origin,
                    [pos[0] + step[0], pos[1] + step[1], pos[2] + step[2]],
                )
            })
    }

    /// The cells of this shape for a Builder at `origin`, in the jar's
    /// `composeFormula` order (x slowest, then z, then y), stopping once `cap`
    /// cells have been kept. `truncated` says the shape holds more.
    #[must_use]
    pub fn cells(self, origin: [i32; 3], cap: u64) -> (Vec<[i32; 3]>, bool) {
        self.cells_within(origin, cap, u64::MAX)
    }

    /// [`Card::cells`] with a bound on how many candidates it may test, so a readout that only
    /// needs a count never walks a shape whose box holds tens of millions of cells.
    #[must_use]
    pub fn cells_within(self, origin: [i32; 3], cap: u64, work: u64) -> (Vec<[i32; 3]>, bool) {
        let min = self.min_corner(origin);
        let dim = self.modifier.rotation.transform_dimension(self.dimension);
        let mut out = Vec::new();
        let mut truncated = false;
        let mut looked = 0u64;
        'outer: for ox in 0..dim[0] {
            for oz in 0..dim[2] {
                for oy in 0..dim[1] {
                    if looked >= work {
                        truncated = true;
                        break 'outer;
                    }
                    looked += 1;
                    let pos = [min[0] + ox, min[1] + oy, min[2] + oz];
                    if !(self.contains(origin, pos)) {
                        continue;
                    }
                    if out.len() as u64 == cap {
                        truncated = true;
                        break 'outer;
                    }
                    out.push(pos);
                }
            }
        }
        (out, truncated)
    }

    /// How many cells this shape holds, counted up to `cap` (so a box answers
    /// in constant time and the others stop early). `truncated` says the shape
    /// holds more than `cap`.
    #[must_use]
    pub fn count(self, cap: u64) -> (u64, bool) {
        self.count_within(cap, u64::MAX)
    }

    /// [`Card::count`] with a bound on the candidates it may inspect, so a readout can answer in
    /// bounded time: it stops once `work` cells have been looked at and reports that there are
    /// more, exactly as it does past `cap` kept cells. A box is answered from its dimensions and
    /// inspects nothing.
    #[must_use]
    pub fn count_within(self, cap: u64, work: u64) -> (u64, bool) {
        if self.modifier == Modifier::default() && self.shape == Shape::Box {
            let volume = self.box_cells();
            if self.solid {
                return (volume.min(cap), volume > cap);
            }
            // Hollow box: the box minus the shell's interior.
            let interior = self
                .clamped()
                .dimension
                .map(|d| u64::from(u32::try_from(d - 2).unwrap_or(0)))
                .into_iter()
                .product::<u64>();
            let shell = volume - interior;
            return (shell.min(cap), shell > cap);
        }
        let (cells, truncated) = self.cells_within([0, 0, 0], cap, work);
        (cells.len() as u64, truncated)
    }
}

/// The number of cells a box of `dim` holds, saturating.
fn cells_of(dim: [i32; 3]) -> u64 {
    dim.into_iter()
        .map(|d| u64::from(u32::try_from(d).unwrap_or(0)))
        .product()
}

/// The jar's shape formulas (`Formulas.*.isInside`), in absolute coordinates:
/// `origin` is the Builder, `dim` the card's dimension and `offset` its offset.
fn inside_shape(
    shape: Shape,
    dim: [i32; 3],
    offset: [i32; 3],
    origin: [i32; 3],
    pos: [i32; 3],
) -> bool {
    let frame = Frame::new(dim, origin, offset);
    match shape {
        Shape::Box => {
            let min = frame.min();
            (0..3).all(|axis| pos[axis] >= min[axis] && pos[axis] < min[axis] + dim[axis])
        }
        Shape::Sphere => frame.sphere(pos, None),
        Shape::TopDome => frame.sphere(pos, Some(true)),
        Shape::BottomDome => frame.sphere(pos, Some(false)),
        Shape::Cylinder => frame.cylinder(pos, false),
        Shape::CappedCylinder => frame.cylinder(pos, true),
        Shape::Cone => frame.cone(pos),
        Shape::Prism => frame.prism(pos),
        Shape::Torus => frame.torus(pos),
        Shape::Heart => frame.heart(pos),
    }
}

/// One shape's box, centre and Builder, as the jar's formulas set them up
/// (`Formulas.*.setup`): `thisCoord` is the Builder's own position.
struct Frame {
    dim: [i32; 3],
    origin: [i32; 3],
    offset: [i32; 3],
    min: [i32; 3],
    centre: [f32; 3],
}

impl Frame {
    fn new(dim: [i32; 3], origin: [i32; 3], offset: [i32; 3]) -> Self {
        let mut min = [0; 3];
        let mut centre = [0.0; 3];
        for axis in 0..3 {
            // Java's `/` truncates towards zero, as Rust's does.
            min[axis] = origin[axis] - dim[axis] / 2 + offset[axis];
            // `xCoord + offset + ((dim % 2 != 0) ? 0.0f : -.5f)`
            centre[axis] = origin[axis] as f32 + offset[axis] as f32 + parity(dim[axis]);
        }
        Self {
            dim,
            origin,
            offset,
            min,
            centre,
        }
    }

    /// The box's low corner.
    fn min(&self) -> [i32; 3] {
        self.min
    }

    /// The position as the formulas' floats.
    fn local(&self, pos: [i32; 3]) -> [f32; 3] {
        [pos[0] as f32, pos[1] as f32, pos[2] as f32]
    }

    /// `((d + factor)²) / 4` per axis.
    fn ellipse(&self, factor: f32) -> [f32; 3] {
        self.dim
            .map(|d| (d as f32 + factor) * (d as f32 + factor) / 4.0)
    }

    /// `(int) ((dx + dy + dz + factor * 3) / 3)`: the sphere's `davg`.
    fn average3(&self, factor: f32) -> i32 {
        let total = self.dim[0] as f32 + self.dim[1] as f32 + self.dim[2] as f32 + factor * 3.0;
        (total / 3.0) as i32
    }

    /// `(int) ((dx + dz + factor * 2) / 2)`: the cylinder's and cone's `davg`.
    fn average2(&self, factor: f32) -> i32 {
        let total = self.dim[0] as f32 + self.dim[2] as f32 + factor * 2.0;
        (total / 2.0) as i32
    }

    /// `Formulas.FormulaSphere` and the two domes: the ellipse test over all
    /// three axes, `half` selecting the upper (`Some(true)`) or lower part.
    fn sphere(&self, pos: [i32; 3], half: Option<bool>) -> bool {
        let y = pos[1] as f32;
        if let Some(top) = half {
            if top && y < self.centre[1] {
                return false;
            }
            if !top && y > self.centre[1] {
                return false;
            }
        }
        let [x, y, z] = self.local(pos);
        let [dx2, dy2, dz2] = self.ellipse(1.8);
        let davg = self.average3(1.8);
        let squared = (x - self.centre[0]) * (x - self.centre[0]) / dx2
            + (y - self.centre[1]) * (y - self.centre[1]) / dy2
            + (z - self.centre[2]) * (z - self.centre[2]) / dz2;
        let distance = f64::from(squared).sqrt();
        ((distance * f64::from(davg / 2 + 1)) as i32) < davg / 2
    }

    /// `Formulas.FormulaCylinder` and `FormulaCappedCylinder`: the ellipse over
    /// x and z only, the capped one clipped to the card's height.
    fn cylinder(&self, pos: [i32; 3], capped: bool) -> bool {
        if capped && !(self.min[1]..self.min[1] + self.dim[1]).contains(&pos[1]) {
            return false;
        }
        let [x, _, z] = self.local(pos);
        let davg = self.average2(1.7);
        let [dx2, _, dz2] = self.ellipse(1.7);
        let squared = (x - self.centre[0]) * (x - self.centre[0]) / dx2
            + (z - self.centre[2]) * (z - self.centre[2]) / dz2;
        let distance = f64::from(squared).sqrt();
        ((distance * f64::from(davg / 2 + 1)) as i32) < davg / 2
    }

    /// `Formulas.FormulaCone`: the ellipse narrowed towards the top.
    fn cone(&self, pos: [i32; 3]) -> bool {
        let [x, y, z] = self.local(pos);
        let davg = self.average2(1.7);
        let [dx2, _, dz2] = self.ellipse(1.7);
        let squared = (x - self.centre[0]) * (x - self.centre[0]) / dx2
            + (z - self.centre[2]) * (z - self.centre[2]) / dz2;
        let distance = f64::from(squared).sqrt();
        let top = self.origin[1] + self.offset[1] + self.dim[1] / 2;
        let height = self.dim[1] as f32 + 0.5;
        let limit = (davg / 2 - 1) as f32 * (top as f32 - y) / height;
        // Java truncates the reach to an int before comparing it with the
        // float limit.
        let reach = (distance * f64::from(davg / 2 + 1)) as i32;
        reach as f32 <= limit
    }

    /// `Formulas.FormulaPrism`: a box narrowing towards the top.
    fn prism(&self, pos: [i32; 3]) -> bool {
        let min = self.min;
        let slice = pos[1] - min[1];
        if !(0..self.dim[1]).contains(&slice) {
            return false;
        }
        pos[0] >= min[0] + slice
            && pos[0] < min[0] + self.dim[0] - slice
            && pos[2] >= min[2] + slice
            && pos[2] < min[2] + self.dim[2] - slice
    }

    /// `Formulas.FormulaTorus`.
    fn torus(&self, pos: [i32; 3]) -> bool {
        let [x, y, z] = self.local(pos);
        let small = (self.dim[1] - 2) as f32 / 2.0;
        let big = (self.dim[0] - 2) as f32 / 2.0 - small;
        let squared = (x - self.centre[0]) * (x - self.centre[0])
            + (z - self.centre[2]) * (z - self.centre[2]);
        let ring = f64::from(big) - f64::from(squared).sqrt();
        let f = ring * ring + f64::from(y - self.centre[1]) * f64::from(y - self.centre[1])
            - f64::from(small) * f64::from(small);
        f < 0.0
    }

    /// `Formulas.FormulaHeart`, whose three constants push the surface
    /// off-centre (so it mirrors on no axis).
    fn heart(&self, pos: [i32; 3]) -> bool {
        let [x, y, z] = self.local(pos);
        let xx = f64::from(x - self.centre[0]) * 2.6 / f64::from(self.dim[0]) + 0.1;
        let zz = f64::from(y - self.centre[1]) * 2.4 / f64::from(self.dim[1]) + 0.2;
        let yy = f64::from(z - self.centre[2]) * 1.6 / f64::from(self.dim[2]) + 0.1;
        let f1 = (xx * xx + 2.25 * yy * yy + zz * zz - 1.0).powi(3);
        let f2 = xx * xx * zz * zz * zz;
        let f3 = (9.0 / 80.0) * yy * yy * zz * zz * zz;
        f1 - f2 - f3 < 0.0
    }
}

/// The jar's `(dim % 2 != 0) ? 0.0f : -.5f`.
fn parity(dim: i32) -> f32 {
    if dim % 2 == 0 { -0.5 } else { 0.0 }
}

/// The one-cell step of the Builder's walk: x fastest, then z, then y from the
/// top down, over the walked box, so every cell of the box is looked at exactly
/// once per pass.
///
/// `None` means the pass completed and the caller restarts at the top.
///
/// **Upstream defect fixed.** The jar's `nextLocationQuarry`
/// (`BuilderTileEntity.java:2198-2228`) steps `x` by one but snaps `x` and `z`
/// **down to the enclosing 16-block chunk** whenever it advances `z` or drops a
/// layer (`(x >> 4) << 4`), the remnant of its quarry chunk-loading. A box
/// whose low corner is in an earlier chunk than its high corner therefore has
/// whole columns that no pass ever looks at: for the default 5×5×5 box at a
/// Builder on a chunk boundary (`min.x = -2, max.x = 3`) the jar's walk reaches
/// **50 of its 125 cells**, and every later pass repeats the same miss, so the
/// other 75 are never built. This walk keeps the jar's top-down, x-fastest
/// order and reaches all 125 (see `the_walk_looks_at_every_cell_of_a_straddling_box`).
#[must_use]
pub fn next_cursor(cursor: [i32; 3], min: [i32; 3], max: [i32; 3]) -> Option<[i32; 3]> {
    let [x, y, z] = cursor;
    if x + 1 < max[0] {
        return Some([x + 1, y, z]);
    }
    if z + 1 < max[2] {
        return Some([min[0], y, z + 1]);
    }
    if y > min[1] {
        return Some([min[0], y - 1, min[2]]);
    }
    None
}

impl Scan {
    /// A scan of `card` for a Builder at `origin`, starting where the jar's
    /// `restartScan` starts a shaped build — the top of the low corner — on the
    /// topmost cell of the box (the jar's cursor sits one block above it, in
    /// the empty layer, and spends its first layer looking at nothing).
    #[must_use]
    pub fn start(origin: [i32; 3], card: Card) -> Self {
        let min = card.min_corner(origin);
        let max = card.max_corner(origin);
        Self {
            card,
            origin,
            min,
            max,
            cursor: Some([min[0], max[1] - 1, min[2]]),
        }
    }

    /// The scan resumed from a saved cursor (or not yet started). A cursor
    /// outside the walked box (a card that changed under a running pass) is
    /// dropped and the pass starts again at the top.
    #[must_use]
    pub fn resume(origin: [i32; 3], card: Card, cursor: Option<[i32; 3]>) -> Self {
        let mut scan = Self::start(origin, card);
        scan.cursor = match cursor {
            None => None,
            Some(cursor)
                if (0..3).all(|axis| {
                    cursor[axis] >= scan.min[axis] && cursor[axis] < scan.max[axis]
                }) =>
            {
                Some(cursor)
            }
            // A card that changed under a running pass: start the pass again.
            Some(_) => scan.cursor,
        };
        scan
    }

    /// Step to the next cell the walk looks at; `false` when the pass finished
    /// and the scan restarted (its cursor is then back at the start).
    pub fn advance(&mut self) -> bool {
        match self
            .cursor
            .and_then(|cursor| next_cursor(cursor, self.min, self.max))
        {
            Some(next) => {
                self.cursor = Some(next);
                true
            }
            None => {
                self.cursor = Some([self.min[0], self.max[1] - 1, self.min[2]]);
                false
            }
        }
    }

    /// Clip the walk's height to the world's build height, as the jar's
    /// `calculateBoxShaped` clips its box: `false` when the shape lies wholly
    /// outside it and there is nothing to build.
    pub fn clamp_y(&mut self, min_y: i32, max_y: i32) -> bool {
        self.min[1] = self.min[1].max(min_y);
        self.max[1] = self.max[1].min(max_y);
        if self.min[1] >= self.max[1] {
            return false;
        }
        if self
            .cursor
            .is_some_and(|cursor| cursor[1] < self.min[1] || cursor[1] >= self.max[1])
        {
            self.cursor = Some([self.min[0], self.max[1] - 1, self.min[2]]);
        }
        true
    }
}

/// The cells of `cells` that lie inside the world's build height, in order.
#[must_use]
pub fn clip_to_height(cells: &[[i32; 3]], min_y: i32, max_y: i32) -> Vec<[i32; 3]> {
    cells
        .iter()
        .copied()
        .filter(|pos| pos[1] >= min_y && pos[1] < max_y)
        .collect()
}

/// Whether `pos` lies inside the world's build height.
#[must_use]
pub fn in_height(pos: [i32; 3], min_y: i32, max_y: i32) -> bool {
    pos[1] >= min_y && pos[1] < max_y
}

impl RefusalReason {
    /// The words the readout and the action bar use.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            RefusalReason::NoMaterial => "no block to place (put one above or below the builder)",
            RefusalReason::OutOfWorld => "outside the world's build height",
            RefusalReason::Protected => "protected (outside the world border)",
        }
    }
}

impl CardError {
    /// The cells the refused card's clamped box holds.
    #[must_use]
    pub fn cells(&self) -> u64 {
        match self {
            CardError::TooLarge { cells, .. } => *cells,
        }
    }

    /// The limit it passed.
    #[must_use]
    pub fn limit(&self) -> u64 {
        match self {
            CardError::TooLarge { limit, .. } => *limit,
        }
    }
}

impl std::fmt::Display for CardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardError::TooLarge { cells, limit } => {
                write!(f, "shape of {cells} cells past the {limit}-cell card limit")
            }
        }
    }
}

/// `BuilderHeader` + the card + the progress, as `/rftoolsbuilder status` says
/// it. Byte-stable: the tests pin every branch.
#[must_use]
pub fn status_line(status: &Status) -> String {
    let [x, y, z] = status.at;
    let Some(card) = status.card else {
        return match &status.refused_card {
            Some(error) => format!(
                "Builder at {x} {y} {z}: card refused, shape of {} cells past the {}-cell card limit",
                error.cells(),
                error.limit()
            ),
            None => format!("Builder at {x} {y} {z}: no shape card"),
        };
    };
    let [dx, dy, dz] = card.dimension;
    let [ox, oy, oz] = card.offset;
    let count = status.cells.unwrap_or_else(|| {
        let (cells, more) = card.count_within(MAX_PASS_CELLS, STATUS_CANDIDATES);
        CellCount { cells, more }
    });
    let (cells, truncated) = (count.cells, count.more);
    let cap = if truncated {
        format!(" (first {cells} cells)")
    } else {
        String::new()
    };
    let head = format!(
        "Builder at {x} {y} {z}: {} ({}) {dx}x{dy}x{dz} offset {ox} {oy} {oz}, {cells} cells{cap}",
        card.shape.description(),
        card.solid_word()
    );
    if let Some(refusal) = status.refusal {
        let [rx, ry, rz] = refusal.at;
        return format!(
            "{head}, {} placed, refused at {rx} {ry} {rz}: {}",
            status.progress.placed,
            refusal.reason.text()
        );
    }
    if status.complete {
        return format!("{head}, {} placed, scan complete", status.progress.placed);
    }
    if status.progress.placed == 0 && status.progress.skipped == 0 {
        return format!("{head}, not started");
    }
    format!(
        "{head}, {} placed, {} skipped",
        status.progress.placed, status.progress.skipped
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Builder at the origin, so a cell reads as its own coordinates.
    const AT: [i32; 3] = [0, 0, 0];

    fn card(shape: Shape, solid: bool, dim: [i32; 3]) -> Card {
        Card {
            shape,
            solid,
            dimension: dim,
            offset: [0, 0, 0],
            modifier: Modifier::default(),
        }
    }

    fn count(shape: Shape, solid: bool, dim: [i32; 3]) -> u64 {
        card(shape, solid, dim).count(u64::MAX).0
    }

    // --- the shapes' exact cell counts ------------------------------------

    /// The tick's walk, exactly as `support::tick` drives it: at most `BLOCKS_PER_TICK` placements
    /// and `LOOKUPS_PER_TICK` cells a tick, one tick at a time, with a fake world that holds the
    /// scenario's own blocks (the Builder, the chest above it, the pedestal) and everything an
    /// earlier pass built. Returns (cells visited in shape, placed, ticks).
    fn walk_pass(
        origin: [i32; 3],
        card: Card,
        occupied: &mut std::collections::BTreeSet<[i32; 3]>,
        ticks: u32,
    ) -> (u64, u64, u32) {
        let mut scan = Scan::start(origin, card);
        let mut visited = 0u64;
        let mut placed = 0u64;
        let mut used = 0u32;
        for _ in 0..ticks {
            used += 1;
            let mut this_tick = 0u32;
            let mut steps = 0u32;
            while this_tick < BLOCKS_PER_TICK && steps < LOOKUPS_PER_TICK {
                let Some(cursor) = scan.cursor else { break };
                steps += 1;
                if card.contains(origin, cursor) {
                    visited += 1;
                    if !occupied.contains(&cursor) {
                        occupied.insert(cursor);
                        placed += 1;
                        this_tick += 1;
                    }
                }
                if !scan.advance() {
                    return (visited, placed, used);
                }
            }
        }
        (visited, placed, used)
    }

    #[test]
    fn a_second_pass_over_a_hollow_box_visits_and_builds_every_cell() {
        // The harness's sequence: a solid 3x3x3 pass on a Builder that has a chest above it and a
        // pedestal under the box's corner, then a hollow 5x5x5 pass on the same Builder. Every cell
        // of the shell must be visited once and built: a walk that under-visits shows up here as
        // fewer placed cells than the shape holds.
        let at = [0, 0, 0];
        let mut occupied: std::collections::BTreeSet<[i32; 3]> = std::collections::BTreeSet::new();
        occupied.insert(at); // the Builder itself
        occupied.insert([0, 1, 0]); // the chest above it
        occupied.insert([0, 0, -1]); // the pedestal under the box's corner

        let solid = Card {
            shape: Shape::Box,
            solid: true,
            dimension: [3, 3, 3],
            ..Card::default()
        };
        let (visited, placed, _) = walk_pass(at, solid, &mut occupied, 200);
        assert_eq!(visited, 27, "every cell of a solid 3x3x3 box");
        assert_eq!(
            placed, 24,
            "the Builder, the chest above it and the pedestal cannot take a block"
        );

        let hollow = Card {
            shape: Shape::Box,
            solid: false,
            dimension: [5, 5, 5],
            ..Card::default()
        };
        let (visited, placed, ticks) = walk_pass(at, hollow, &mut occupied, 400);
        assert_eq!(visited, 98, "every shell cell is looked at exactly once");
        assert_eq!(
            placed, 98,
            "and every shell cell is built (none of them is the chest, the Builder or the pedestal)"
        );
        assert!(ticks < 400, "the pass finishes inside its ticks");
    }

    #[test]
    fn a_box_holds_exactly_its_dimensions() {
        assert_eq!(count(Shape::Box, true, [3, 3, 3]), 27);
        assert_eq!(count(Shape::Box, true, [5, 5, 5]), 125);
        assert_eq!(count(Shape::Box, true, [1, 1, 1]), 1);
        let box3 = card(Shape::Box, true, [3, 3, 3]);
        assert_eq!(box3.min_corner(AT), [-1, -1, -1]);
        assert_eq!(box3.max_corner(AT), [2, 2, 2]);
        assert!(box3.contains(AT, [-1, -1, -1]));
        assert!(!box3.contains(AT, [2, 2, 2]));
    }

    #[test]
    fn a_hollow_box_is_its_shell() {
        assert_eq!(count(Shape::Box, false, [3, 3, 3]), 26);
        assert_eq!(count(Shape::Box, false, [5, 5, 5]), 98);
        let hollow = card(Shape::Box, false, [3, 3, 3]);
        assert!(hollow.contains(AT, [-1, -1, -1]), "a corner is shell");
        assert!(
            !hollow.contains(AT, [0, 0, 0]),
            "the middle of a 3×3×3 box is not"
        );
    }

    #[test]
    fn the_sphere_is_eighty_one_cells_at_five() {
        assert_eq!(count(Shape::Sphere, true, [5, 5, 5]), 81);
        // A sphere is never a full cube: at 3 the ellipse test keeps only the
        // centre, the six face neighbours and the twelve edges (19), not 27.
        assert_eq!(count(Shape::Sphere, true, [3, 3, 3]), 19);
    }

    #[test]
    fn the_domes_are_the_spheres_halves() {
        assert_eq!(count(Shape::TopDome, true, [5, 5, 5]), 51);
        assert_eq!(count(Shape::BottomDome, true, [5, 5, 5]), 51);
        let top = card(Shape::TopDome, true, [5, 5, 5]);
        assert!(top.contains(AT, [0, 2, 0]));
        assert!(!top.contains(AT, [0, -2, 0]));
    }

    #[test]
    fn the_cylinder_ignores_its_height_and_the_capped_one_clips_it() {
        assert_eq!(count(Shape::Cylinder, true, [5, 5, 5]), 105);
        assert_eq!(count(Shape::CappedCylinder, true, [5, 5, 5]), 105);
        assert_eq!(count(Shape::Cone, true, [5, 5, 5]), 21);
        assert_eq!(count(Shape::Prism, true, [5, 5, 5]), 35);
        assert_eq!(count(Shape::Torus, true, [5, 5, 5]), 19);
    }

    #[test]
    fn hollow_cylinders_keep_the_jars_heightless_border_test() {
        // `isBorder` asks `isInside` of the six neighbours, and the cylinder's
        // `isInside` ignores y, so the vertical neighbours always read as
        // inside: a hollow cylinder has no cap at all, only its rim, whatever
        // its height.
        assert_eq!(count(Shape::Cylinder, false, [5, 1, 5]), 12);
        assert_eq!(count(Shape::Cylinder, false, [5, 3, 5]), 36);
        // The capped cylinder's `isInside` does look at y, so it does cap.
        assert_eq!(count(Shape::CappedCylinder, false, [5, 1, 5]), 21);
    }

    #[test]
    fn the_sphere_mirrors_on_every_axis() {
        let sphere = card(Shape::Sphere, true, [7, 7, 7]);
        let (cells, _) = sphere.cells(AT, u64::MAX);
        assert!(cells.len() > 100, "a 7×7×7 sphere is not tiny");
        for [x, y, z] in cells {
            for mirror in [[-x, y, z], [x, -y, z], [x, y, -z]] {
                assert!(
                    sphere.contains(AT, mirror),
                    "sphere loses {mirror:?} while holding {}, {y}, {z}",
                    x
                );
            }
        }
    }

    #[test]
    fn the_heart_keeps_the_jars_uneven_formula() {
        // The heart's three constants (`+ .1`, `+ .2`, `+ .1`) push its surface
        // off-centre, so it mirrors on no axis; a port that dropped one would
        // gain a mirror here.
        let heart = card(Shape::Heart, true, [9, 9, 9]);
        let (cells, _) = heart.cells(AT, u64::MAX);
        assert_eq!(cells.len(), 247);
        let broken = |mirror: fn([i32; 3]) -> [i32; 3]| {
            cells
                .iter()
                .filter(|pos| !heart.contains(AT, mirror(**pos)))
                .count()
        };
        assert!(broken(|[x, y, z]| [-x, y, z]) > 0, "x is nudged by +.1");
        assert!(broken(|[x, y, z]| [x, -y, z]) > 0, "y is nudged by +.2");
        assert!(broken(|[x, y, z]| [x, y, -z]) > 0, "z is nudged by +.1");
    }

    #[test]
    fn cells_come_in_the_jars_x_z_y_order() {
        let (cells, _) = card(Shape::Box, true, [2, 2, 2]).cells(AT, u64::MAX);
        assert_eq!(cells.len(), 8);
        assert_eq!(cells[0], [-1, -1, -1]);
        assert_eq!(cells[1], [-1, 0, -1], "y varies fastest, as composeFormula");
        assert_eq!(cells[2], [-1, -1, 0], "then z");
        assert_eq!(cells[4], [0, -1, -1], "then x");
    }

    // --- the offset, the clamps and the caps -------------------------------

    #[test]
    fn the_offset_moves_the_shape_and_not_the_builder() {
        let moved = Card {
            offset: [10, 0, 0],
            ..card(Shape::Box, true, [3, 3, 3])
        };
        assert_eq!(moved.min_corner(AT), [9, -1, -1]);
        let (cells, _) = moved.cells(AT, u64::MAX);
        assert!(cells.iter().all(|pos| (9..12).contains(&pos[0])));
        assert_eq!(cells.len(), 27);
    }

    #[test]
    fn an_offset_leaving_the_world_places_nothing() {
        let below = Card {
            offset: [0, -200, 0],
            ..card(Shape::Box, true, [3, 3, 3])
        };
        let (cells, _) = below.cells([0, 100, 0], u64::MAX);
        assert_eq!(cells.len(), 27);
        assert!(
            clip_to_height(&cells, -64, 320).is_empty(),
            "every cell is under the world"
        );
        assert!(!in_height(cells[0], -64, 320));
        // A shape half in and half out keeps only what the world holds.
        let straddling = Card {
            offset: [0, -4, 0],
            ..card(Shape::Box, true, [5, 5, 5])
        };
        let (cells, _) = straddling.cells([0, -62, 0], u64::MAX);
        let kept = clip_to_height(&cells, -64, 320);
        assert_eq!(cells.len(), 125);
        assert!(kept.len() < cells.len());
        assert!(kept.iter().all(|pos| pos[1] >= -64));
    }

    #[test]
    fn every_card_is_clamped_to_the_jars_numbers() {
        let huge = Card {
            dimension: [1000, 4, 4],
            offset: [900, -900, 0],
            ..card(Shape::Box, true, [1, 1, 1])
        }
        .clamped();
        assert_eq!(huge.dimension, [512, 4, 4]);
        assert_eq!(huge.offset, [260, -260, 0]);
        assert_eq!(MAX_DIMENSION, 512);
        assert_eq!(MAX_OFFSET, 260);
    }

    #[test]
    fn a_card_past_the_limit_is_refused_with_the_jars_numbers() {
        let too_big = Card {
            dimension: [1000, 1000, 1000],
            ..card(Shape::Box, true, [1, 1, 1])
        };
        let error = too_big.validate().expect_err("512³ is past 50M");
        assert_eq!(
            error,
            CardError::TooLarge {
                cells: 134_217_728,
                limit: 50_000_000
            }
        );
        assert_eq!(MAX_CARD_CELLS, 50_000_000);
        assert_eq!(
            error.to_string(),
            "shape of 134217728 cells past the 50000000-cell card limit"
        );
        assert!(card(Shape::Box, true, [100, 100, 100]).validate().is_ok());
        assert!(
            card(Shape::Box, true, [400, 400, 400]).validate().is_err(),
            "64M cells is past the card limit"
        );
    }

    #[test]
    fn a_pass_keeps_at_most_the_space_chambers_cell_cap() {
        let big = card(Shape::Box, true, [200, 200, 200]);
        assert_eq!(big.box_cells(), 8_000_000);
        let (kept, truncated) = big.count(MAX_PASS_CELLS);
        assert_eq!(kept, MAX_PASS_CELLS);
        assert!(truncated);
        assert_eq!(MAX_PASS_CELLS, 128 * 128 * 128);
        let (cells, truncated) = big.cells(AT, 10);
        assert_eq!(cells.len(), 10);
        assert!(truncated);
    }

    // --- the card's parsing ------------------------------------------------

    #[test]
    fn a_card_parses_the_jars_shape_names() {
        assert_eq!(Shape::parse("box"), Some(Shape::Box));
        assert_eq!(Shape::parse("SHAPE_BOX"), Some(Shape::Box));
        assert_eq!(Shape::parse("Box"), Some(Shape::Box));
        assert_eq!(Shape::parse("top dome"), Some(Shape::TopDome));
        assert_eq!(Shape::parse("SHAPE_TOPDOME"), Some(Shape::TopDome));
        assert_eq!(Shape::parse("cappedcylinder"), Some(Shape::CappedCylinder));
        assert_eq!(
            Shape::parse("SHAPE_CAPPEDCYLINDER"),
            Some(Shape::CappedCylinder)
        );
        assert_eq!(Shape::parse("sphere"), Some(Shape::Sphere));
        assert_eq!(Shape::parse("heart"), Some(Shape::Heart));
        assert_eq!(Shape::parse("nonsense"), None);
        assert_eq!(Shape::parse("SHAPE_COMPOSITION"), None, "not carried");
        assert_eq!(Shape::parse("SHAPE_SCAN"), None, "not carried");
        assert_eq!(Shape::Box.id(), "SHAPE_BOX");
        assert_eq!(Shape::Box.description(), "Box");
        assert_eq!(Rotation::parse("0"), Some(Rotation::None));
        assert_eq!(Rotation::parse("y"), Some(Rotation::Y));
        assert_eq!(Rotation::parse("Q"), None);
        assert_eq!(Rotation::Y.transform_dimension([7, 3, 5]), [5, 3, 7]);
    }

    // --- the walk ----------------------------------------------------------

    #[test]
    fn the_walk_steps_x_then_z_then_down() {
        let min = [0, 0, 0];
        let max = [3, 3, 3];
        // x first.
        assert_eq!(next_cursor([0, 2, 0], min, max), Some([1, 2, 0]));
        // The row done: z first, x back to the box's own low corner.
        assert_eq!(next_cursor([2, 2, 0], min, max), Some([0, 2, 1]));
        // The layer done: down, x and z back to the low corner.
        assert_eq!(next_cursor([2, 2, 2], min, max), Some([0, 1, 0]));
        // The box done: the pass is over.
        assert_eq!(next_cursor([2, 0, 2], min, max), None);
    }

    #[test]
    fn the_walk_looks_at_every_cell_of_a_straddling_box() {
        // The default 5×5×5 box for a Builder on a chunk boundary: the jar's
        // chunk-snapping walk reaches only 50 of these 125 cells, on every
        // pass (notes.md, the defect this walk fixes).
        let card = card(Shape::Box, true, [5, 5, 5]);
        let mut scan = Scan::start(AT, card);
        let mut seen = Vec::new();
        loop {
            seen.push(scan.cursor.expect("a running scan has a cursor"));
            if !scan.advance() {
                break;
            }
        }
        let (cells, _) = card.cells(AT, u64::MAX);
        seen.retain(|pos| card.contains(AT, *pos));
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(cells.len(), 125);
        assert_eq!(seen.len(), 125, "every cell is looked at in one pass");
        for cell in &cells {
            assert!(seen.contains(cell), "{cell:?} is never looked at");
        }
        let start = Scan::start(AT, card);
        assert_eq!(scan.cursor, start.cursor, "the pass restarts at the top");
    }

    #[test]
    fn a_scan_starts_on_the_top_cell_of_the_low_corner() {
        let scan = Scan::start([10, 64, 10], card(Shape::Box, true, [3, 3, 3]));
        assert_eq!(scan.min, [9, 63, 9]);
        assert_eq!(scan.max, [12, 66, 12]);
        assert_eq!(scan.cursor, Some([9, 65, 9]));
    }

    #[test]
    fn a_resumed_cursor_outside_the_box_starts_the_pass_again() {
        let card = card(Shape::Box, true, [3, 3, 3]);
        let inside = Scan::resume(AT, card, Some([0, 0, 0]));
        assert_eq!(inside.cursor, Some([0, 0, 0]));
        let outside = Scan::resume(AT, card, Some([40, 40, 40]));
        assert_eq!(outside.cursor, Scan::start(AT, card).cursor);
        let lower = Scan::resume(AT, card, Some([0, -40, 0]));
        assert_eq!(lower.cursor, Scan::start(AT, card).cursor);
    }

    #[test]
    fn a_scan_is_clipped_to_the_worlds_build_height() {
        // Under the world: nothing left to build.
        let mut under = Scan::start([0, -70, 0], card(Shape::Box, true, [5, 5, 5]));
        assert_eq!(under.min[1], -72);
        assert!(!under.clamp_y(-64, 320));
        // Half under: only the part the world holds is walked.
        let mut half = Scan::start([0, -64, 0], card(Shape::Box, true, [5, 5, 5]));
        assert!(half.clamp_y(-64, 320));
        assert_eq!(half.min, [-2, -64, -2]);
        assert_eq!(half.max, [3, -61, 3]);
        let mut seen = Vec::new();
        loop {
            seen.push(half.cursor.expect("a running scan has a cursor"));
            if !half.advance() {
                break;
            }
        }
        seen.retain(|pos| card(Shape::Box, true, [5, 5, 5]).contains([0, -64, 0], *pos));
        assert!(!seen.is_empty());
        assert!(seen.iter().all(|pos| pos[1] >= -64), "{seen:?}");
    }

    // --- the turn and the mirror -------------------------------------------

    #[test]
    fn the_mirror_turns_the_cone_up_side_down() {
        let cone = card(Shape::Cone, true, [5, 5, 5]);
        let flipped = Card {
            modifier: Modifier {
                rotation: Rotation::None,
                flip_y: true,
            },
            ..cone
        };
        assert!(
            cone.contains(AT, [1, -2, 0]),
            "the cone's base is three wide"
        );
        assert!(
            !flipped.contains(AT, [1, -2, 0]),
            "the mirror puts the wide end at the top"
        );
        assert_eq!(flipped.count(u64::MAX).0, cone.count(u64::MAX).0);
    }

    #[test]
    fn a_turn_keeps_a_box_and_turns_a_heart() {
        // `FormulaComposition` clips to the *turned* bounds; a box fills its
        // turned box (5 × 3 × 7), so turning it changes nothing.
        let turned = Card {
            modifier: Modifier {
                rotation: Rotation::Y,
                flip_y: false,
            },
            ..card(Shape::Box, true, [7, 3, 5])
        };
        assert_eq!(turned.min_corner(AT), [-2, -1, -3]);
        assert_eq!(turned.max_corner(AT), [3, 2, 4]);
        assert_eq!(turned.count(u64::MAX).0, 105);
        assert!(!turned.contains(AT, [-3, 0, 0]), "outside the turned box");
        // A shape that is not symmetric does change under the turn.
        let heart = card(Shape::Heart, true, [9, 9, 9]);
        let spun = Card {
            modifier: Modifier {
                rotation: Rotation::X,
                flip_y: false,
            },
            ..heart
        };
        let (plain, _) = heart.cells(AT, u64::MAX);
        let (rotated, _) = spun.cells(AT, u64::MAX);
        assert!(!plain.is_empty() && !rotated.is_empty());
        assert_ne!(plain, rotated, "a turn must turn an uneven shape");
        assert!(rotated.iter().all(|pos| {
            (0..3).all(|axis| {
                pos[axis] >= spun.min_corner(AT)[axis] && pos[axis] < spun.max_corner(AT)[axis]
            })
        }));
    }

    // --- the readout -------------------------------------------------------

    #[test]
    fn the_status_line_is_byte_stable() {
        let at = [10, 64, 10];
        assert_eq!(
            status_line(&Status {
                at,
                card: None,
                refused_card: None,
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: None,
            }),
            "Builder at 10 64 10: no shape card"
        );
        let sphere = Card {
            shape: Shape::Sphere,
            ..Card::default()
        };
        assert_eq!(
            status_line(&Status {
                at,
                card: Some(sphere),
                refused_card: None,
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: None,
            }),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 81 cells, not started"
        );
        assert_eq!(
            status_line(&Status {
                at,
                card: Some(sphere),
                refused_card: None,
                progress: Progress {
                    placed: 12,
                    skipped: 40
                },
                refusal: None,
                complete: false,
                cells: None,
            }),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 81 cells, 12 placed, 40 skipped"
        );
        assert_eq!(
            status_line(&Status {
                at,
                card: Some(sphere),
                refused_card: None,
                progress: Progress {
                    placed: 81,
                    skipped: 0
                },
                refusal: None,
                complete: true,
                cells: None,
            }),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 81 cells, 81 placed, scan complete"
        );
        assert_eq!(
            status_line(&Status {
                at,
                card: Some(sphere),
                refused_card: None,
                progress: Progress {
                    placed: 12,
                    skipped: 3
                },
                refusal: Some(Refusal {
                    at: [10, 68, 10],
                    reason: RefusalReason::NoMaterial,
                }),
                complete: false,
                cells: None,
            }),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 81 cells, 12 placed, \
             refused at 10 68 10: no block to place (put one above or below the builder)"
        );
        assert_eq!(
            status_line(&Status {
                at,
                card: None,
                refused_card: Some(CardError::TooLarge {
                    cells: 134_217_728,
                    limit: MAX_CARD_CELLS,
                }),
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: None,
            }),
            "Builder at 10 64 10: card refused, shape of 134217728 cells past the 50000000-cell card limit"
        );
    }

    #[test]
    fn a_status_count_is_bounded_on_a_huge_sparse_shape() {
        // A 499x499x499 sphere is sparse enough that a shell of candidates is mostly outside the
        // shape. Counting all of them is ~124 million `contains` calls on the server thread, which
        // is what the review measured; the readout now stops after STATUS_CANDIDATES candidates.
        let sphere = card(Shape::Sphere, true, [499, 499, 499]);
        let (shown, more) = sphere.count_within(MAX_PASS_CELLS, STATUS_CANDIDATES);
        assert!(more, "the answer says the shape holds more");
        assert!(shown <= STATUS_CANDIDATES, "it counted {shown} cells");
        let small = card(Shape::Sphere, true, [199, 199, 199]);
        let (whole, _) = small.count(MAX_PASS_CELLS);
        let (small_shown, small_more) = small.count_within(MAX_PASS_CELLS, STATUS_CANDIDATES);
        assert!(
            small_more || small_shown <= whole,
            "never more than the whole shape"
        );
        // The work bound is what stops it: with a slightly larger budget the same shape counts
        // more cells, so the number is the budget's, not a constant.
        let (bigger, _) = sphere.count_within(MAX_PASS_CELLS, STATUS_CANDIDATES * 2);
        assert!(
            bigger > shown,
            "a bigger budget counts more: {bigger} vs {shown}"
        );
        // A box answers from its dimensions, exactly, and never inspects a cell: with no work
        // budget at all it still answers.
        let boxed = card(Shape::Box, true, [300, 300, 300]);
        let (cells, more) = boxed.count_within(MAX_PASS_CELLS, 0);
        assert_eq!(cells, MAX_PASS_CELLS);
        assert!(more);
    }

    #[test]
    fn the_readout_prints_the_count_it_was_given() {
        // A cached count (the pass computed it once) is printed as it stands, so a repeated
        // readout never re-walks the shape.
        let sphere = Card {
            shape: Shape::Sphere,
            ..Card::default()
        };
        let with_cache = |cells: u64, more: bool| {
            status_line(&Status {
                at: [10, 64, 10],
                card: Some(sphere),
                refused_card: None,
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: Some(CellCount { cells, more }),
            })
        };
        assert_eq!(
            with_cache(81, false),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 81 cells, not started"
        );
        assert_eq!(
            with_cache(1000, true),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 1000 cells (first 1000 cells), \
             not started"
        );
    }

    #[test]
    fn a_protected_cell_has_words_a_player_reads() {
        assert_eq!(
            RefusalReason::Protected.text(),
            "protected (outside the world border)"
        );
    }

    #[test]
    fn the_refusal_reasons_are_the_words_a_player_reads() {
        assert_eq!(
            RefusalReason::NoMaterial.text(),
            "no block to place (put one above or below the builder)"
        );
        assert_eq!(
            RefusalReason::OutOfWorld.text(),
            "outside the world's build height"
        );
        assert_eq!(BLOCKS_PER_TICK, 8);
        assert_eq!(LOOKUPS_PER_TICK, 800);
    }

    #[test]
    fn a_truncated_shape_says_so_in_the_readout() {
        let big = Card {
            dimension: [300, 300, 300],
            ..Card::default()
        };
        assert_eq!(
            status_line(&Status {
                at: [0, 64, 0],
                card: Some(big),
                refused_card: None,
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: None,
            }),
            "Builder at 0 64 0: Box (Solid) 300x300x300 offset 0 0 0, 2097152 cells \
             (first 2097152 cells), not started"
        );
    }
}
