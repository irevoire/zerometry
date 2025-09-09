use std::ops;

/// This struct is used to query the specific relationship between two shapes.
/// By default nothing is enabled and no relation are computed.
///
/// The difference between the strict and normal version of contains and contained are when dealing with multi-shape.
/// Contains would return true if only one point of a multi-poins is contained in the first shape.
/// The strict contains only returns true if all the points of the multi-points are contained in the
/// first shape. It's also way more expensive to compute.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InputRelation {
    /// Return true if any part on the first shape contains any part of the second shape.
    pub contains: bool,
    /// Return true if any parts of the first shape contains all parts of the second shape.
    pub strict_contains: bool,

    /// Return true if any part of the first shape is contained in any part of the second shape.
    pub contained: bool,
    /// Return true if all parts of the first shape are contained in any part of the second shape.
    pub strict_contained: bool,

    /// Return true if all parts of the first shape are contained in any part of the second shape.
    pub intersect: bool,

    /// Return true if there is no relation between both shapes.
    pub disjoint: bool,

    /// If set to `true` the relation algorithm will stop as soon as possible after filling any value.
    /// For example if you are asking if a shape contains, is contained or intersect with another but
    /// don't really care about which of these happened you can set `early_exit` to true and the relation
    /// algorithm will be able to exit directly after finding the first intersection for example.
    pub early_exit: bool,
}

impl InputRelation {
    /// Set everything to `true` and cannot early exit.
    pub fn all() -> Self {
        Self {
            contains: true,
            strict_contains: true,
            contained: true,
            strict_contained: true,
            intersect: true,
            disjoint: true,
            early_exit: false,
        }
    }

    /// Set everything to `true` but can early exit.
    pub fn any() -> Self {
        Self {
            contains: true,
            strict_contains: true,
            contained: true,
            strict_contained: true,
            intersect: true,
            disjoint: true,
            early_exit: true,
        }
    }

    /// Swap the contains and contained relation.
    pub fn swap_contains_relation(mut self) -> Self {
        std::mem::swap(&mut self.contains, &mut self.contained);
        std::mem::swap(&mut self.strict_contains, &mut self.strict_contained);
        self
    }

    /// Set everything to false, same as [`Self::default`].
    pub fn none() -> Self {
        Self::default()
    }

    /// Generates an [`OutputRelation`] where every `true` field of `Self` are set to `Some(false)`.
    pub fn to_false(self) -> OutputRelation {
        OutputRelation::false_from_input(self)
    }

    /// Generates an [`OutputRelation`] where every `true` field of `Self` are set to `Some(true)`.
    pub fn to_true(self) -> OutputRelation {
        OutputRelation::true_from_input(self)
    }

    /// Remove the strict contains and contained.
    pub fn strip_strict(mut self) -> Self {
        self.strict_contains = false;
        self.strict_contained = false;
        self
    }

    /// Remove only the strict contained.
    pub fn strip_strict_contained(mut self) -> Self {
        self.strict_contained = false;
        self
    }

    /// Remove disjoint.
    pub fn strip_disjoint(mut self) -> Self {
        self.disjoint = false;
        self
    }
}

/// Returned by the `relation` function.
/// All fields are made of a `Option<bool>`.
/// There are two cases for which a field can be None:
/// - If you didn't ask for it when filling the `InputRelation` struct
/// - If the relation algorithm didn't evaluate this relation because the
///   `early_exit` flag was set.
///
/// Note that when early exit is set, most fields will be set to `Some(false)` even
/// though they were not evaluated at all.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OutputRelation {
    /// Return true if any part on the first shape contains any part of the second shape.
    pub contains: Option<bool>,
    /// Return true if any parts of the first shape contains all parts of the second shape.
    pub strict_contains: Option<bool>,
    /// Return true if any part of the first shape is contained in any part of the second shape.
    pub contained: Option<bool>,
    /// Return true if all parts of the first shape are contained in any part of the second shape.
    pub strict_contained: Option<bool>,
    /// Return true if all parts of the first shape are contained in any part of the second shape.
    pub intersect: Option<bool>,
    /// Return true if there is no relation between both shapes.
    pub disjoint: Option<bool>,
}

impl OutputRelation {
    pub(crate) fn false_from_input(relation: InputRelation) -> Self {
        Self {
            contains: relation.contains.then_some(false),
            strict_contains: relation.strict_contains.then_some(false),
            contained: relation.contained.then_some(false),
            strict_contained: relation.strict_contained.then_some(false),
            intersect: relation.intersect.then_some(false),
            disjoint: relation.disjoint.then_some(false),
        }
    }

    pub(crate) fn true_from_input(relation: InputRelation) -> Self {
        Self {
            contains: relation.contains.then_some(true),
            strict_contains: relation.strict_contains.then_some(true),
            contained: relation.contained.then_some(true),
            strict_contained: relation.strict_contained.then_some(true),
            intersect: relation.intersect.then_some(true),
            disjoint: relation.disjoint.then_some(true),
        }
    }

    pub(crate) fn make_contains_if_set(mut self) -> Self {
        self.contains = self.contains.map(|_| true);
        self
    }

    /// Set both the contains and strict_contains field to true if they are set
    pub(crate) fn make_strict_contains_if_set(mut self) -> Self {
        self.strict_contains = self.strict_contains.map(|_| true);
        self.make_contains_if_set()
    }

    pub(crate) fn make_contained_if_set(mut self) -> Self {
        self.contained = self.contained.map(|_| true);
        self
    }

    /// Set both the contained and strict_contained field to true if they are set
    pub(crate) fn make_strict_contained_if_set(mut self) -> Self {
        self.strict_contained = self.strict_contained.map(|_| true);
        self.make_contained_if_set()
    }

    pub(crate) fn make_intersect_if_set(mut self) -> Self {
        self.intersect = self.intersect.map(|_| true);
        self
    }

    pub(crate) fn make_disjoint_if_set(mut self) -> Self {
        self.disjoint = self.disjoint.map(|_| true);
        self
    }

    pub(crate) fn strip_strict(mut self) -> Self {
        self.strict_contains = None;
        self.strict_contained = None;
        self
    }

    /// Return true if the output contains anything except disjoint.
    pub fn any_relation(&self) -> bool {
        // If the shape are distinct we don't need to check anything else and can stop early
        (!self.disjoint.unwrap_or_default())
            // otherwise we must check every single entry and return true if any contains a true
            && (self.contains.unwrap_or_default()
                || self.strict_contains.unwrap_or_default()
                || self.contained.unwrap_or_default()
                || self.strict_contained.unwrap_or_default()
                || self.intersect.unwrap_or_default())
    }

    /// Swap the contains and contained relation.
    pub fn swap_contains_relation(mut self) -> Self {
        std::mem::swap(&mut self.contains, &mut self.contained);
        std::mem::swap(&mut self.strict_contains, &mut self.strict_contained);
        self
    }
}

impl ops::BitOr for OutputRelation {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        let Self {
            mut contains,
            mut strict_contains,
            mut contained,
            mut strict_contained,
            mut intersect,
            mut disjoint,
        } = self;

        if let Some(ref mut s) = contains {
            *s |= other.contains.unwrap_or_default()
        }

        if let Some(ref mut s) = strict_contains {
            *s |= other.strict_contains.unwrap_or_default()
        }

        if let Some(ref mut s) = contained {
            *s |= other.contained.unwrap_or_default()
        }

        if let Some(ref mut s) = strict_contained {
            *s |= other.strict_contained.unwrap_or_default()
        }

        if let Some(ref mut s) = intersect {
            *s |= other.intersect.unwrap_or_default()
        }

        if let Some(ref mut s) = disjoint {
            *s |= other.disjoint.unwrap_or_default()
        }

        Self {
            contains,
            strict_contains,
            contained,
            strict_contained,
            intersect,
            disjoint,
        }
    }
}

impl ops::BitOrAssign for OutputRelation {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

/// Lets you query the relation between two shapes.
pub trait RelationBetweenShapes<Other: ?Sized> {
    /// Return the relation between two shapes.
    /// The [`InputRelation`] lets you specify the kind of relation you want to retrieve.
    ///
    // ```
    /// use zerometry::{Zoint, Zolygon, RelationBetweenShapes, InputRelation};
    ///
    /// let point = geo_types::Point::new(0.0, 0.0);
    /// let polygon = geo_types::polygon![(x: -1.0, y: -1.0), (x: 1.0, y: -1.0), (x: 1.0, y: 1.0), (x: -1.0, y: 1.0)];
    ///
    /// let mut buffer = Vec::new();
    /// Zoint::write_from_geometry(&mut buffer, &point).unwrap();
    /// let zoint = Zoint::from_bytes(&buffer);
    /// let mut buffer = Vec::new();
    /// Zoint::write_from_geometry(&mut buffer, &polygon).unwrap();
    /// let zolygon = Zolygon::from_bytes(&buffer);
    ///
    /// // Let's say we just want to know if the point is contained in the polygon,
    /// // we could write
    /// let relation = InputRelation { contains: true, ..InputRelation::default() };
    /// // The we can ask the relation between our two shape with the `relation` method:
    /// let relation = zolygon.relation(&zoint, relation);
    /// assert_eq!(relation.contains, Some(true));
    /// ```
    fn relation(&self, other: &Other, relation: InputRelation) -> OutputRelation;

    /// Return all relations with no early return.
    fn all_relation(&self, other: &Other) -> OutputRelation {
        self.relation(other, InputRelation::all())
    }

    /// Return the first relation we find with early return.
    fn any_relation(&self, other: &Other) -> OutputRelation {
        self.relation(other, InputRelation::any())
    }

    /// Return `true` if `Self` contains `Other`.
    fn contains(&self, other: &Other) -> bool {
        self.relation(
            other,
            InputRelation {
                contains: true,
                early_exit: true,
                ..Default::default()
            },
        )
        .contains
        .unwrap_or_default()
    }

    /// Return `true` if `Self` strictly contains `Other`.
    fn strict_contains(&self, other: &Other) -> bool {
        self.relation(
            other,
            InputRelation {
                strict_contains: true,
                early_exit: true,
                ..Default::default()
            },
        )
        .strict_contains
        .unwrap_or_default()
    }

    /// Return `true` if `Self` is contained in `Other`.
    fn contained(&self, other: &Other) -> bool {
        self.relation(
            other,
            InputRelation {
                contained: true,
                early_exit: true,
                ..Default::default()
            },
        )
        .contained
        .unwrap_or_default()
    }

    /// Return `true` if `Self` is strictly contained in `Other`.
    fn strict_contained(&self, other: &Other) -> bool {
        self.relation(
            other,
            InputRelation {
                strict_contained: true,
                early_exit: true,
                ..Default::default()
            },
        )
        .strict_contained
        .unwrap_or_default()
    }

    /// Return `true` if `Self` intersects with `Other`.
    fn intersects(&self, other: &Other) -> bool {
        self.relation(
            other,
            InputRelation {
                intersect: true,
                early_exit: true,
                ..Default::default()
            },
        )
        .intersect
        .unwrap_or_default()
    }

    /// Return `true` if `Self` is disjoint of `Other`.
    fn disjoint(&self, other: &Other) -> bool {
        self.relation(
            other,
            InputRelation {
                disjoint: true,
                early_exit: true,
                ..Default::default()
            },
        )
        .disjoint
        .unwrap_or_default()
    }
}
