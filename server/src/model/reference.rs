use std::marker::PhantomData;

use impl_tools::autoimpl;
use serde::{Deserialize, Serialize};

use self::sealed::Sealed;

use super::{Flow, Policy, Prompt, Stage};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[autoimpl(Hash ignore self._target)]
#[autoimpl(PartialEq ignore self._target)]
#[autoimpl(Eq)]
pub struct Reference<Target: ReferenceTarget> {
    #[serde(flatten)]
    pub id: ReferenceId,
    #[serde(skip)]
    _target: PhantomData<Target>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum ReferenceKind {
    Uid,
    Slug,
}

impl<Target: ReferenceTarget> Reference<Target> {
    pub const fn new(id: ReferenceId) -> Self {
        Self {
            id,
            _target: PhantomData,
        }
    }

    pub const fn new_uid(id: i32) -> Self {
        Self::new(ReferenceId::Uid(id))
    }
    pub const fn new_slug(slug: String) -> Self {
        Self::new(ReferenceId::Slug(slug))
    }

    pub const fn kind(&self) -> ReferenceKind {
        match self.id {
            ReferenceId::Slug(_) => ReferenceKind::Slug,
            ReferenceId::Uid(_) => ReferenceKind::Uid,
        }
    }
}

mod sealed {
    use crate::model::{Flow, Policy, Prompt, Stage};

    pub trait Sealed {}
    impl Sealed for Flow {}
    impl Sealed for Stage {}
    impl Sealed for Prompt {}
    impl Sealed for Policy {}
}

pub trait ReferenceTarget: Sealed {
    type Value;
}

pub trait Referencable: ReferenceTarget + Sized {
    fn ref_uid(&self) -> Option<Reference<Self>>;
    fn ref_slug(&self) -> Option<Reference<Self>>;
}

macro_rules! ref_target {
    ($ty:ty) => {
        impl ReferenceTarget for $ty {
            type Value = Self;
        }
    };
}

ref_target!(Stage);
ref_target!(Prompt);
ref_target!(Policy);
ref_target!(Flow);

impl Referencable for Stage {
    fn ref_uid(&self) -> Option<Reference<Self>> {
        Some(Reference::new_uid(self.uid))
    }

    fn ref_slug(&self) -> Option<Reference<Self>> {
        Some(Reference::new_slug(self.slug.clone()))
    }
}

impl Referencable for Prompt {
    fn ref_uid(&self) -> Option<Reference<Self>> {
        Some(Reference::new_uid(self.uid))
    }

    fn ref_slug(&self) -> Option<Reference<Self>> {
        None
    }
}

impl Referencable for Policy {
    fn ref_uid(&self) -> Option<Reference<Self>> {
        Some(Reference::new_uid(self.uid))
    }

    fn ref_slug(&self) -> Option<Reference<Self>> {
        Some(Reference::new_slug(self.slug.clone()))
    }
}

impl Referencable for Flow {
    fn ref_uid(&self) -> Option<Reference<Self>> {
        None
    }

    fn ref_slug(&self) -> Option<Reference<Self>> {
        Some(Reference::new_slug(self.slug.clone()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum ReferenceId {
    Slug(String),
    Uid(i32),
}
