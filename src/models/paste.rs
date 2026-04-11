//! Paste object and related items.

use chrono::Utc;
use sqlx::{PgExecutor, Postgres, QueryBuilder, Row as _};

use crate::{
    app::database::Database,
    models::{
        DtUtc,
        errors::{AuthenticationError, RESTError},
        undefined::{Undefined, UndefinedOption},
    },
};

use super::{authentication::Token, errors::DatabaseError, snowflake::Snowflake};

/// ## Paste
///
/// The paste object stored in the database.
#[derive(Debug, Clone)]
pub struct Paste {
    /// The ID of the paste.
    id: Snowflake,
    /// The pastes name.
    name: Option<String>,
    /// When the paste was created.
    creation: DtUtc,
    /// When the paste was last modified.
    edited: Option<DtUtc>,
    /// The time at which the paste will expire.
    expiry: Option<DtUtc>,
    /// The amount of views a paste has.
    views: usize,
    /// The maximum allowed views for a paste.
    max_views: Option<usize>,
}

impl Paste {
    /// New.
    ///
    /// Create a new [`Paste`] object.
    pub const fn new(
        id: Snowflake,
        name: Option<String>,
        creation: DtUtc,
        edited: Option<DtUtc>,
        expiry: Option<DtUtc>,
        views: usize,
        max_views: Option<usize>,
    ) -> Self {
        Self {
            id,
            name,
            creation,
            edited,
            expiry,
            views,
            max_views,
        }
    }

    /// The pastes ID.
    #[inline]
    pub const fn id(&self) -> &Snowflake {
        &self.id
    }

    /// The pastes name.
    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The pastes creation time.
    #[inline]
    pub const fn creation(&self) -> &DtUtc {
        &self.creation
    }

    /// The pastes last edited time.
    #[inline]
    pub const fn edited(&self) -> Option<&DtUtc> {
        self.edited.as_ref()
    }

    /// The pastes expiry time.
    #[inline]
    pub const fn expiry(&self) -> Option<&DtUtc> {
        self.expiry.as_ref()
    }

    /// The pastes total view count.
    #[inline]
    pub const fn views(&self) -> usize {
        self.views
    }

    /// The pastes maximum allowed views.
    #[inline]
    pub const fn max_views(&self) -> Option<usize> {
        self.max_views
    }

    /// Fetch.
    ///
    /// Fetch a paste via its ID.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    /// - `id` - The ID of the paste.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// - [`Option::Some`] - The [`Paste`] object.
    /// - [`Option::None`] - No paste was found.
    pub async fn fetch<'e, 'c: 'e, E>(
        executor: E,
        id: &Snowflake,
    ) -> Result<Option<Self>, DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = (*id).into();
        let query = sqlx::query!(
            "SELECT id, name, creation, edited, expiry, views, max_views FROM pastes WHERE id = $1",
            paste_id
        )
        .fetch_optional(executor)
        .await?;

        if let Some(q) = query {
            return Ok(Some(Self::new(
                q.id.into(),
                q.name,
                q.creation,
                q.edited,
                q.expiry,
                q.views as usize,
                q.max_views.map(|v| v as usize),
            )));
        }

        Ok(None)
    }

    /// Fetch Between.
    ///
    /// Fetch all pastes between two times.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    /// - `start` - The start [`DtUtc`] (inclusive).
    /// - `end` - The end [`DtUtc`] (inclusive).
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// A [`Vec`] of [`Paste`]'s.
    pub async fn fetch_between<'e, 'c: 'e, E>(
        executor: E,
        start: &DtUtc,
        end: &DtUtc,
    ) -> Result<Vec<Self>, DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let records = sqlx::query!(
            "SELECT id, name, creation, edited, expiry, views, max_views FROM pastes WHERE expiry >= $1 AND expiry <= $2",
            start,
            end
        )
        .fetch_all(executor)
        .await?;

        let mut pastes = Vec::new();
        for record in records {
            let paste = Self::new(
                record.id.into(),
                record.name,
                record.creation,
                record.edited,
                record.expiry,
                record.views as usize,
                record.max_views.map(|v| v as usize),
            );

            pastes.push(paste);
        }

        Ok(pastes)
    }

    /// Insert.
    ///
    /// Insert (create) a paste.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error, or the snowflake exists already.
    pub async fn insert<'e, 'c: 'e, E>(&self, executor: E) -> Result<(), DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = self.id.into();

        sqlx::query!(
            "INSERT INTO pastes(id, name, creation, edited, expiry, views, max_views) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            paste_id,
            self.name,
            self.creation,
            self.edited,
            self.expiry,
            self.views as i64,
            self.max_views.map(|v| v as i64)
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Update.
    ///
    /// Create (or update) a document.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    pub async fn update<'e, 'c: 'e, E>(
        &mut self,
        executor: E,
        parameters: PasteUpdateParameters,
    ) -> Result<bool, DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        if parameters.is_empty() {
            return Ok(false);
        }

        let id_val: i64 = self.id.into();

        let mut builder: QueryBuilder<'_, Postgres> =
            sqlx::QueryBuilder::new("UPDATE pastes SET edited = ");
        builder.push_bind(Utc::now());

        if !parameters.name().is_undefined() {
            let value: Option<&str> = parameters.name().into();

            builder.push(", name = ");
            builder.push_bind(value);
        }

        if !parameters.expiry().is_undefined() {
            let value: Option<&DtUtc> = parameters.expiry().into();

            builder.push(", expiry = ");
            builder.push_bind(value);
        }

        if let Undefined::Some(size) = parameters.views() {
            builder.push(", views = ");
            builder.push_bind(size as i64);
        }

        if !parameters.max_views().is_undefined() {
            let value: Option<usize> = parameters.max_views().into();

            builder.push(", max_views = ");
            builder.push_bind(value.map(|v| v as i64));
        }

        builder.push(" WHERE id = ");
        builder.push_bind(id_val);
        builder.push(" RETURNING *");

        let record = builder.build().fetch_one(executor).await?;
        println!("Here 1");

        self.edited = record.get("edited");
        self.name = record.get("name");
        self.expiry = record.get("expiry");
        let views: i64 = record.get("views");
        self.views = views as usize;
        let max_views: Option<i64> = record.get("max_views");
        self.max_views = max_views.map(|v| v as usize);

        Ok(true)
    }

    /// Add view.
    ///
    /// Increment a pastes view count by 1.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    /// - `id` - The ID of the paste to add the view to.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    pub async fn add_view<'e, 'c: 'e, E>(&mut self, executor: E) -> Result<(), DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let id_val: i64 = self.id.into();

        let views = sqlx::query_scalar!(
            "UPDATE pastes SET views = views + 1 WHERE id = $1 RETURNING views",
            id_val,
        )
        .fetch_one(executor)
        .await?;

        self.views = views as usize;

        Ok(())
    }

    /// Delete.
    ///
    /// Delete a paste.
    ///
    /// ## Arguments
    ///
    /// - `executor` - The database pool or transaction to use.
    /// - `id` - The id of the paste.
    ///
    /// ## Errors
    ///
    /// - [`DatabaseError`] - The database had an error.
    pub async fn delete<'e, 'c: 'e, E>(executor: E, id: &Snowflake) -> Result<bool, DatabaseError>
    where
        E: 'e + PgExecutor<'c>,
    {
        let paste_id: i64 = (*id).into();
        let result = sqlx::query!("DELETE FROM pastes WHERE id = $1", paste_id,)
            .execute(executor)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

/// ## Paste Update Parameters
///
/// The parameters that can be used to update a paste.
pub struct PasteUpdateParameters {
    name: UndefinedOption<String>,
    expiry: UndefinedOption<DtUtc>,
    views: Undefined<usize>,
    max_views: UndefinedOption<usize>,
}

impl PasteUpdateParameters {
    /// ## New.
    ///
    /// Create a new [`PasteUpdateParameters`] object.
    pub const fn new(
        name: UndefinedOption<String>,
        expiry: UndefinedOption<DtUtc>,
        views: Undefined<usize>,
        max_views: UndefinedOption<usize>,
    ) -> Self {
        Self {
            name,
            expiry,
            views,
            max_views,
        }
    }

    /// The name to update the paste with.
    pub fn name(&self) -> UndefinedOption<&str> {
        self.name.as_deref()
    }

    /// The expiry to update the paste with.
    pub const fn expiry(&self) -> UndefinedOption<&DtUtc> {
        self.expiry.as_ref()
    }

    /// The views to update the paste with.
    pub const fn views(&self) -> Undefined<usize> {
        self.views
    }

    /// The maximum views to update the paste with.
    pub const fn max_views(&self) -> UndefinedOption<usize> {
        self.max_views
    }

    /// ## Is Empty
    ///
    /// Used to check if the update parameters updates nothing.
    ///
    /// ## Returns
    /// Returns [`true`] if all parameters are undefined, otherwise returns [`false`].
    pub const fn is_empty(&self) -> bool {
        self.name.is_undefined()
            && self.expiry.is_undefined()
            && self.views.is_undefined()
            && self.max_views.is_undefined()
    }
}

/// Validate Paste.
///
/// Checks that a paste exists, and has not expired,
/// as well as supporting validating the token.
///
/// ## Arguments
///
/// - `db` - The database to use.
/// - `paste_id` - The ID of the paste.
/// - `token` - The token to validate (if required.)
///
/// ## Errors
///
/// - [`DatabaseError`] - The database had an error.
///
/// ## Returns
///
/// The paste that was checked and found.
pub async fn validate_paste(
    db: &Database,
    paste_id: &Snowflake,
    token: Option<Token>,
) -> Result<Paste, RESTError> {
    let Some(paste) = Paste::fetch(db.pool(), paste_id).await? else {
        return Err(RESTError::NotFound(
            "The paste requested could not be found".to_string(),
        ));
    };

    if let Some(expiry) = paste.expiry
        && expiry < Utc::now()
    {
        Paste::delete(db.pool(), paste_id).await?;
        return Err(RESTError::NotFound(
            "The paste requested could not be found".to_string(),
        ));
    }

    if let Some(max_views) = paste.max_views
        && paste.views >= max_views
    {
        Paste::delete(db.pool(), paste_id).await?;
        return Err(RESTError::NotFound(
            "The paste requested could not be found".to_string(),
        ));
    }

    if let Some(token) = token
        && paste.id != *token.paste_id()
    {
        return Err(RESTError::Authentication(
            AuthenticationError::InvalidCredentials,
        ));
    }

    Ok(paste)
}
