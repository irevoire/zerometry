use std::ops;

/// This struct is used to query the specific relationship between two shapes.
/// By default nothing is enabled and no relation are computed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InputRelation {
    /// Return true if any part on the first shape contains any part of the second shape.
    pub contains: bool,
    /// Return true if any parts of the first shape contains all parts of the second shape.
    /// The difference between the strict and lose contains are when dealing with multi-shape.
    /// Contains would return true if only one point of a multi-poins is contained in the first shape.
    /// The strict contains only returns true if all the points of the multi-points are contained in the
    /// first shape. It's also way more expensive to compute.
    pub strict_contains: bool,

    /// Return true if any part on the first shape is contained in any part of the second shape.
    pub contained: bool,
    pub strict_contained: bool,

    pub intersect: bool,

    pub disjoint: bool,

    /// If set to `true` the relation algorithm will stop as soon as possible after filling any value.
    /// For example if you are asking if a shape contains, is contained or intersect with another but
    /// don't really care about which of these happened you can set `early_exit` to true and the relation
    /// algorithm will be able to exit directly after finding the first intersection for example.
    pub early_exit: bool,
}

impl InputRelation {
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

    pub fn swap_contains_relation(mut self) -> Self {
        std::mem::swap(&mut self.contains, &mut self.contained);
        std::mem::swap(&mut self.strict_contains, &mut self.strict_contained);
        self
    }

    pub fn none() -> Self {
        Self::default()
    }

    pub fn to_false(self) -> OutputRelation {
        OutputRelation::false_from_input(self)
    }

    pub fn strip_strict(mut self) -> Self {
        self.strict_contains = false;
        self.strict_contained = false;
        self
    }

    pub fn strip_strict_contained(mut self) -> Self {
        self.strict_contained = false;
        self
    }

    pub fn strip_disjoint(mut self) -> Self {
        self.disjoint = false;
        self
    }
}

/// Returned by the `relation` function.
/// All fields are made of a Option<bool>.
/// There are two cases for which a field can be None:
/// - If you didn't ask for it when filling the `InputRelation` struct
/// - If the relation algorithm didn't evaluate this relation because the
///   `early_exit` flag was set.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OutputRelation {
    pub contains: Option<bool>,
    pub strict_contains: Option<bool>,
    pub contained: Option<bool>,
    pub strict_contained: Option<bool>,
    pub intersect: Option<bool>,
    pub disjoint: Option<bool>,
}

impl OutputRelation {
    pub fn false_from_input(relation: InputRelation) -> Self {
        Self {
            contains: relation.contains.then_some(false),
            strict_contains: relation.strict_contains.then_some(false),
            contained: relation.contained.then_some(false),
            strict_contained: relation.strict_contained.then_some(false),
            intersect: relation.intersect.then_some(false),
            disjoint: relation.disjoint.then_some(false),
        }
    }

    pub fn make_contains_if_set(mut self) -> Self {
        self.contains = self.contains.map(|_| true);
        self
    }

    /// Set both the contains and strict_contains field to true if they are set
    pub fn make_strict_contains_if_set(mut self) -> Self {
        self.strict_contains = self.strict_contains.map(|_| true);
        self.make_contains_if_set()
    }

    pub fn make_contained_if_set(mut self) -> Self {
        self.contained = self.contained.map(|_| true);
        self
    }

    /// Set both the contained and strict_contained field to true if they are set
    pub fn make_strict_contained_if_set(mut self) -> Self {
        self.strict_contained = self.strict_contained.map(|_| true);
        self.make_contained_if_set()
    }

    pub fn make_intersect_if_set(mut self) -> Self {
        self.intersect = self.intersect.map(|_| true);
        self
    }

    pub fn make_disjoint_if_set(mut self) -> Self {
        self.disjoint = self.disjoint.map(|_| true);
        self
    }

    pub fn contains() -> Self {
        Self {
            contains: Some(true),
            ..Default::default()
        }
    }

    pub fn strict_contains() -> Self {
        Self {
            contains: Some(true),
            strict_contains: Some(true),
            ..Default::default()
        }
    }

    pub fn contained() -> Self {
        Self {
            contained: Some(true),
            ..Default::default()
        }
    }

    pub fn strict_contained() -> Self {
        Self {
            contained: Some(true),
            strict_contained: Some(true),
            ..Default::default()
        }
    }

    pub fn intersect() -> Self {
        Self {
            intersect: Some(true),
            ..Default::default()
        }
    }

    pub fn disjoint() -> Self {
        Self {
            disjoint: Some(true),
            ..Default::default()
        }
    }

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

    fn strip_unrequested_fields(self, relation: InputRelation) -> Self {
        let Self {
            mut contains,
            mut strict_contains,
            mut contained,
            mut strict_contained,
            mut intersect,
            mut disjoint,
        } = self;

        if !relation.contains {
            contains = None;
        }

        if !relation.strict_contains {
            strict_contains = None;
        }

        if !relation.contained {
            contained = None;
        }
        if !relation.strict_contained {
            strict_contained = None;
        }
        if !relation.intersect {
            intersect = None;
        }
        if !relation.disjoint {
            disjoint = None;
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

pub trait RelationBetweenShapes<Other: ?Sized> {
    fn relation(&self, other: &Other, relation: InputRelation) -> OutputRelation;

    fn all_relation(&self, other: &Other) -> OutputRelation {
        self.relation(other, InputRelation::all())
    }

    fn any_relation(&self, other: &Other) -> OutputRelation {
        self.relation(other, InputRelation::any())
    }

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
