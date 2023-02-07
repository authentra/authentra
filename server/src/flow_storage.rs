use std::{collections::HashMap, fmt::Debug, ops::Deref, sync::Arc};

use async_trait::async_trait;
use futures_util::future::join_all;
use parking_lot::{Mutex, RwLock};

use sqlx::{query, query_as, FromRow, PgPool};

use crate::model::{
    AuthenticationRequirement, ConsentMode, Flow, FlowBinding, FlowBindingKind, FlowDesignation,
    FlowEntry, PasswordBackend, Policy, PolicyKind, Prompt, PromptKind, Referencable, Reference,
    ReferenceId, ReferenceTarget, Stage, StageKind, UserField,
};

#[derive(Debug)]
struct LookupTable<T> {
    slugs: HashMap<String, Arc<T>>,
    ids: HashMap<i32, Arc<T>>,
}

impl<T> Default for LookupTable<T> {
    fn default() -> Self {
        Self {
            slugs: HashMap::new(),
            ids: HashMap::new(),
        }
    }
}

impl<T: ReferenceTarget + Referencable + Debug> LookupTable<T> {
    pub fn lookup(&self, r: &Reference<T>) -> Option<Arc<T>> {
        match &r.id {
            ReferenceId::Slug(slug) => self.slugs.get(slug),
            ReferenceId::Uid(id) => self.ids.get(id),
        }
        .map(|v| Arc::clone(v))
    }
    pub fn insert(&mut self, value: Arc<T>) {
        if let Some(Reference { id, .. }) = value.ref_uid() {
            self.insert_ref(id, value.clone());
        }
        if let Some(Reference { id, .. }) = value.ref_slug() {
            self.insert_ref(id, value.clone());
        }
    }

    fn insert_ref(&mut self, id: ReferenceId, value: Arc<T>) {
        match id.clone() {
            ReferenceId::Slug(slug) => self.slugs.insert(slug, value),
            ReferenceId::Uid(id) => self.ids.insert(id, value),
        };
    }
}

#[derive(Clone)]
pub struct FlowStorage {
    internal: Arc<FlowStorageInternal>,
}

impl FlowStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            internal: Arc::new(FlowStorageInternal::new(pool)),
        }
    }
}

pub struct FlowStorageInternal {
    stages: RwLock<LookupTable<Stage>>,
    policies: RwLock<LookupTable<Policy>>,
    prompts: RwLock<LookupTable<Prompt>>,
    flows: RwLock<LookupTable<Flow>>,
    pool: Mutex<PgPool>,
}

impl FlowStorageInternal {
    pub fn new(pool: PgPool) -> Self {
        Self {
            stages: RwLock::new(LookupTable::default()),
            policies: RwLock::new(LookupTable::default()),
            prompts: RwLock::new(LookupTable::default()),
            flows: RwLock::new(LookupTable::default()),
            pool: Mutex::new(pool),
        }
    }
}

#[async_trait]
pub trait ReferenceLookup<T: ReferenceTarget> {
    async fn lookup_reference(&self, reference: &Reference<T>) -> Option<Arc<T>>;
}
#[async_trait]
trait ReferenceDbQuery<T: ReferenceTarget> {
    async fn query_reference(&self, reference: &Reference<T>) -> Option<T>;
}

macro_rules! flow_storage_reference_lookup {
    ($ty:ty, $field:ident) => {
        #[async_trait]
        impl ReferenceLookup<$ty> for FlowStorage {
            async fn lookup_reference(&self, reference: &Reference<$ty>) -> Option<Arc<$ty>> {
                match self.internal.$field.read().lookup(reference) {
                    None => self.internal.query_reference(reference).await.map(Arc::new),
                    v => v,
                }
            }
        }
    };
}

flow_storage_reference_lookup!(Stage, stages);
flow_storage_reference_lookup!(Policy, policies);
flow_storage_reference_lookup!(Prompt, prompts);
flow_storage_reference_lookup!(Flow, flows);

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "policy_kind", rename_all = "snake_case")]
pub enum PgPolicyKind {
    PasswordExpiry,
    PasswordStrength,
    Expression,
}

#[derive(FromRow)]
pub struct PgPolicy {
    uid: i32,
    slug: String,
    kind: PgPolicyKind,
    password_expiration: Option<i32>,
    #[allow(unused)]
    password_strength: Option<i32>,
    #[allow(unused)]
    expression: Option<i32>,
}

#[async_trait]
impl ReferenceDbQuery<Policy> for FlowStorageInternal {
    async fn query_reference(&self, reference: &Reference<Policy>) -> Option<Policy> {
        let lock = self.pool.lock();
        let Some(res) = match &reference.id {
            ReferenceId::Slug(slug) => query_as!(
                PgPolicy,
                r#"select uid, slug,kind as "kind: PgPolicyKind",password_expiration,password_strength,expression from policies where slug = $1"#,
                slug
            ).fetch_optional(&*lock).await,
            ReferenceId::Uid(uid) => query_as!(
                PgPolicy,
                r#"select uid, slug,kind as "kind: PgPolicyKind",password_expiration,password_strength,expression from policies where uid = $1"#,
                uid
            ).fetch_optional(&*lock).await,
        }.ok().flatten() else { return None; };

        let kind = match res.kind {
            PgPolicyKind::PasswordExpiry => {
                let Some(uid) = res.password_expiration else { return None };
                let res = query!(
                    "select max_age from password_expiration_policies where uid = $1",
                    uid
                )
                .fetch_one(&*lock)
                .await
                .expect("Failed to fetch password expiration policy")
                .max_age;
                PolicyKind::PasswordExpiry { max_age: res }
            }
            PgPolicyKind::PasswordStrength => PolicyKind::PasswordStrength,
            PgPolicyKind::Expression => PolicyKind::Expression,
        };
        Some(Policy {
            uid: res.uid,
            slug: res.slug,
            kind,
        })
    }
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "stage_kind", rename_all = "snake_case")]
pub enum PgStageKind {
    Deny,
    Prompt,
    Identification,
    UserLogin,
    UserLogout,
    UserWrite,
    Password,
    Consent,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "prompt_kind", rename_all = "snake_case")]
pub enum PgPromptKind {
    Username,
    Email,
    Password,
    Text,
    TextReadOnly,
    SignedNumber,
    UnsignedNumber,
    Checkbox,
    Switch,
    Date,
    DateTime,
    Seperator,
    Static,
    Locale,
}

#[derive(FromRow)]
pub struct PgStage {
    uid: i32,
    slug: String,
    kind: PgStageKind,
    timeout: i64,
    identification_password_stage: Option<i32>,
    identification_stage: Option<i32>,
    consent_stage: Option<i32>,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "consent_mode", rename_all = "snake_case")]
pub enum PgConsentMode {
    Always,
    Once,
    Until,
}

#[async_trait]
impl ReferenceDbQuery<Stage> for FlowStorageInternal {
    async fn query_reference(&self, reference: &Reference<Stage>) -> Option<Stage> {
        let lock = self.pool.lock();
        let Some(res) = match &reference.id {
            ReferenceId::Slug(slug) => query_as!(PgStage,
                r#"select uid, slug, kind as "kind: PgStageKind", timeout, identification_password_stage, consent_stage, identification_stage from stages where slug = $1"#,
                slug
            ).fetch_optional(&*lock).await,
            ReferenceId::Uid(id) => query_as!(PgStage,
                r#"select uid, slug, kind as "kind: PgStageKind", timeout, identification_password_stage, consent_stage, identification_stage from stages where uid = $1"#,
                id
                ).fetch_optional(&*lock).await,
        }.ok().flatten() else { return None };
        let kind = match res.kind {
            PgStageKind::Deny => StageKind::Deny,
            PgStageKind::Prompt => todo!(),
            PgStageKind::Identification => {
                let stage = match query!(
                    r#"select fields as "fields: Vec<UserField>" from identification_stages where uid = $1"#,
                    res.identification_stage
                ).fetch_optional(&*lock).await.ok().flatten() {
                    Some(v) => v,
                    None => return None
                };
                StageKind::Identification {
                    password: res.identification_password_stage.map(Reference::new_uid),
                    user_fields: stage.fields,
                }
            }
            PgStageKind::UserLogin => StageKind::UserLogin,
            PgStageKind::UserLogout => StageKind::UserLogout,
            PgStageKind::UserWrite => StageKind::UserWrite,
            PgStageKind::Password => StageKind::Password {
                backends: vec![PasswordBackend::Internal],
            },
            PgStageKind::Consent => {
                let Some(uid) = res.consent_stage else { return None };
                let Some(res) = query!(
                    r#"select mode as "mode: PgConsentMode", until from consent_stages where uid = $1"#,
                    uid
                ).fetch_one(&*lock).await.ok() else { return None };
                let mode = match res.mode {
                    PgConsentMode::Always => ConsentMode::Always,
                    PgConsentMode::Once => ConsentMode::Once,
                    PgConsentMode::Until => ConsentMode::Until {
                        duration: res.until.unwrap_or(0),
                    },
                };
                StageKind::Consent { mode }
            }
        };
        Some(Stage {
            uid: res.uid,
            slug: res.slug,
            kind,
            timeout: res.timeout,
        })
    }
}

#[async_trait]
impl ReferenceDbQuery<Prompt> for FlowStorageInternal {
    async fn query_reference(&self, reference: &Reference<Prompt>) -> Option<Prompt> {
        let ReferenceId::Uid(uid) = reference.id else { return None };
        let Some(res) = query!(
            r#"select uid, field_key, label, kind as "kind: PgPromptKind", placeholder, required, help_text from prompts where uid = $1"#,
            uid
        ).fetch_one(&*self.pool.lock()).await.ok() else { return None };
        let kind = match res.kind {
            PgPromptKind::Username => PromptKind::Username,
            PgPromptKind::Email => PromptKind::Email,
            PgPromptKind::Password => PromptKind::Password,
            PgPromptKind::Text => PromptKind::Text,
            PgPromptKind::TextReadOnly => PromptKind::TextReadOnly,
            PgPromptKind::SignedNumber => PromptKind::SignedNumber,
            PgPromptKind::UnsignedNumber => PromptKind::UnsignedNumber,
            PgPromptKind::Checkbox => PromptKind::Checkbox,
            PgPromptKind::Switch => PromptKind::Switch,
            PgPromptKind::Date => PromptKind::Date,
            PgPromptKind::DateTime => PromptKind::DateTime,
            PgPromptKind::Seperator => PromptKind::Seperator,
            PgPromptKind::Static => PromptKind::Static,
            PgPromptKind::Locale => PromptKind::Locale,
        };
        Some(Prompt {
            uid: res.uid,
            field_key: res.field_key,
            label: res.label,
            kind,
            placeholder: res.placeholder,
            required: res.required,
            help_text: res.help_text,
        })
    }
}

#[derive(FromRow)]
pub struct PgFlow {
    uid: i32,
    slug: String,
    title: String,
    authentication: AuthenticationRequirement,
    designation: FlowDesignation,
}

#[async_trait]
impl ReferenceDbQuery<Flow> for FlowStorageInternal {
    async fn query_reference(&self, reference: &Reference<Flow>) -> Option<Flow> {
        let lock = self.pool.lock();
        let Some(res) = match &reference.id {
            ReferenceId::Uid(uid) => {
                query_as!(PgFlow, r#"select uid, slug, title, authentication as "authentication: AuthenticationRequirement", designation as "designation: FlowDesignation" from flows where uid = $1"#, uid)
                    .fetch_one(&*lock)
                    .await
            }
            ReferenceId::Slug(slug) => {
                query_as!(PgFlow, r#"select uid, slug, title, authentication as "authentication: AuthenticationRequirement", designation as "designation: FlowDesignation" from flows where slug = $1"#, slug)
                    .fetch_one(&*lock)
                    .await
            }
        }
        .ok() else { return None };
        let bindings = match load_bindings_flow(&*lock, res.uid).await {
            Some(v) => v,
            None => return None,
        };
        let entries = query!(
            "select uid,stage,ordering from flow_entries where flow = $1",
            res.uid
        )
        .fetch_all(&*lock)
        .await;
        let entries = match entries.ok() {
            Some(entries) => {
                let _v = "";
                let db_entries =
                    join_all(entries.iter().map(|v| load_bindings_entry(&*lock, v.uid)))
                        .await
                        .into_iter();
                let entries = entries
                    .into_iter()
                    .zip(db_entries)
                    .filter_map(|(entry, bindings)| bindings.map(|b| (entry, b)))
                    .map(|(entry, bindings)| FlowEntry {
                        ordering: entry.ordering,
                        bindings,
                        stage: Reference::new_uid(entry.stage),
                    });
                entries
            }
            None => return None,
        };
        Some(Flow {
            uid: res.uid,
            slug: res.slug,
            title: res.title,
            designation: res.designation,
            authentication: res.authentication,
            bindings,
            entries: entries.collect(),
        })
    }
}

async fn load_bindings_flow(pool: &PgPool, uid: i32) -> Option<Vec<FlowBinding>> {
    let Some(policies) = query!(
            "select policy,enabled,negate_result,ordering from flow_bindings where flow = $1",
            uid
        )
        .fetch_all(pool)
        .await.ok().map(|v| v.into_iter().map(|v| FlowBinding {
                enabled: v.enabled,
                negate: v.negate_result,
                order: v.ordering,
                kind: FlowBindingKind::Policy(Reference::new_uid(v.policy))
            })) else { return None };

    let Some(users) = query!(
            "select user_binding,enabled,negate_result,ordering from flow_bindings where flow = $1 and user_binding is not null",
            uid
        )
        .fetch_all(pool)
        .await.ok().map(|v| v.into_iter().map(|v| FlowBinding {
                enabled: v.enabled,
                negate: v.negate_result,
                order: v.ordering,
                kind: FlowBindingKind::User(v.user_binding.expect("Selected not null but is null (User)"))
            })) else { return None };
    let Some(groups) = query!(
            "select group_binding,enabled,negate_result,ordering from flow_bindings where flow = $1 and group_binding is not null",
            uid
        )
        .fetch_all(pool)
        .await
        .ok()
        .map(|v| v.into_iter().map(|v| FlowBinding {
                enabled: v.enabled,
                negate: v.negate_result,
                order: v.ordering,
                kind: FlowBindingKind::Group(v.group_binding.expect("Selected not null but is null (Group)"))
            })) else { return None };
    let mut bindings = Vec::new();
    bindings.extend(policies);
    bindings.extend(users);
    bindings.extend(groups);
    bindings.sort_by_key(|v| v.order);
    Some(bindings)
}
async fn load_bindings_entry(pool: &PgPool, uid: i32) -> Option<Vec<FlowBinding>> {
    let Some(policies) = query!(
            "select policy,enabled,negate_result,ordering from flow_bindings where entry = $1",
            uid
        )
        .fetch_all(pool)
        .await.ok().map(|v| v.into_iter().map(|v| FlowBinding {
                enabled: v.enabled,
                negate: v.negate_result,
                order: v.ordering,
                kind: FlowBindingKind::Policy(Reference::new_uid(v.policy))
            })) else { return None };

    let Some(users) = query!(
            "select user_binding,enabled,negate_result,ordering from flow_bindings where entry = $1 and user_binding is not null",
            uid
        )
        .fetch_all(pool)
        .await.ok().map(|v| v.into_iter().map(|v| FlowBinding {
                enabled: v.enabled,
                negate: v.negate_result,
                order: v.ordering,
                kind: FlowBindingKind::User(v.user_binding.expect("Selected not null but is null (User)"))
            })) else { return None };
    let Some(groups) = query!(
            "select group_binding,enabled,negate_result,ordering from flow_bindings where entry = $1 and group_binding is not null",
            uid
        )
        .fetch_all(pool)
        .await
        .ok()
        .map(|v| v.into_iter().map(|v| FlowBinding {
                enabled: v.enabled,
                negate: v.negate_result,
                order: v.ordering,
                kind: FlowBindingKind::Group(v.group_binding.expect("Selected not null but is null (Group)"))
            })) else { return None };
    let mut bindings = Vec::new();
    bindings.extend(policies);
    bindings.extend(users);
    bindings.extend(groups);
    bindings.sort_by_key(|v| v.order);
    Some(bindings)
}

#[derive(Debug, Clone)]
pub struct UnresolvedReference {
    pub kind: UnresolvedReferenceKind,
    pub id: ReferenceId,
}

#[derive(Debug, Clone)]
pub enum UnresolvedReferenceKind {
    Stage,
    Policy,
    Prompt,
    Flow,
}

pub struct FreezedStorage {
    storage: Option<FlowStorage>,
    errors: Option<Mutex<Vec<UnresolvedReference>>>,
    stages: RwLock<LookupTable<Stage>>,
    policies: RwLock<LookupTable<Policy>>,
    prompts: RwLock<LookupTable<Prompt>>,
    flows: RwLock<LookupTable<Flow>>,
}

impl Debug for FreezedStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FreezedStorage")
            .field("stages", &self.stages)
            .finish()
    }
}

impl FreezedStorage {
    pub fn new(storage: FlowStorage) -> Self {
        Self {
            storage: Some(storage),
            errors: Some(Mutex::new(Vec::new())),
            stages: RwLock::new(LookupTable::default()),
            policies: RwLock::new(LookupTable::default()),
            prompts: RwLock::new(LookupTable::default()),
            flows: RwLock::new(LookupTable::default()),
        }
    }
    pub fn freeze(&mut self) -> Vec<UnresolvedReference> {
        if self.storage.is_none() {
            return vec![];
        }
        self.storage = None;
        std::mem::replace(&mut self.errors, None)
            .expect("already freezed")
            .lock()
            .deref()
            .to_vec()
    }
}

macro_rules! freezed_reference_lookup {
    ($ty:ty, $field:ident, $kind:expr) => {
        #[async_trait]
        impl ReferenceLookup<$ty> for FreezedStorage {
            async fn lookup_reference(&self, reference: &Reference<$ty>) -> Option<Arc<$ty>> {
                if let Some(storage) = &self.storage {
                    let value = storage.lookup_reference(reference).await;
                    if let Some(value) = value {
                        self.$field.write().insert(value.clone());
                        return Some(value);
                    }
                    if let Some(errors) = &self.errors {
                        errors.lock().push(UnresolvedReference {
                            kind: $kind,
                            id: reference.id.clone(),
                        });
                    }
                    None
                } else {
                    self.$field.read().lookup(reference)
                }
            }
        }
    };
}

freezed_reference_lookup!(Stage, stages, UnresolvedReferenceKind::Stage);
freezed_reference_lookup!(Policy, policies, UnresolvedReferenceKind::Policy);
freezed_reference_lookup!(Prompt, prompts, UnresolvedReferenceKind::Prompt);
freezed_reference_lookup!(Flow, flows, UnresolvedReferenceKind::Flow);

#[async_trait]
pub trait ReverseLookup {
    async fn reverse_lookup(&self, storage: &FreezedStorage);
}

#[async_trait]
impl ReverseLookup for Flow {
    async fn reverse_lookup(&self, storage: &FreezedStorage) {
        storage
            .lookup_reference(&self.ref_uid().expect("ref_uid() not implemented"))
            .await;
        join_all(self.bindings.iter().map(|binding| async {
            if let FlowBindingKind::Policy(policy) = &binding.kind {
                storage.lookup_reference(policy).await;
            }
        }))
        .await;
        for entry in &self.entries {
            join_all(entry.bindings.iter().map(|binding| async {
                if let FlowBindingKind::Policy(policy) = &binding.kind {
                    storage.lookup_reference(policy).await;
                }
            }))
            .await;
            let stage = storage.lookup_reference(&entry.stage).await;
            if let Some(stage) = stage {
                stage.reverse_lookup(storage).await;
            }
        }
    }
}
#[async_trait]
impl ReverseLookup for Stage {
    async fn reverse_lookup(&self, storage: &FreezedStorage) {
        match &self.kind {
            StageKind::Prompt { bindings } => {
                join_all(bindings.iter().map(|v| async {
                    storage.lookup_reference(&v.prompt).await;
                }))
                .await;
            }
            StageKind::Identification { password, .. } => {
                if let Some(password) = password {
                    let stage = storage.lookup_reference(password).await;
                    if let Some(stage) = stage {
                        stage.reverse_lookup(storage).await;
                    }
                }
            }
            _ => {}
        };
    }
}
