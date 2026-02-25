use sqlx::{FromRow, PgPool};

use crate::repo::{
    EnrollmentLogRecord, EnrollmentMetadataRecord, EnrollmentResetRecord, ExternalId, ImageRecord,
    ProfileRecord, RegistrationErrorRecord, RepoError, RepoResult,
};

#[derive(Clone)]
pub struct SqlxFrRepository {
    pool: PgPool,
}

impl SqlxFrRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_profile(&self, profile: &ProfileRecord) -> RepoResult<()> {
        sqlx::query(
            r#"
            insert into eyefr.profiles (ext_id, last_name, first_name, middle_name, img_url, raw_data, fr_id)
            values ($1, $2, $3, $4, $5, $6, $7)
            on conflict (ext_id) do update
            set
                last_name = excluded.last_name,
                first_name = excluded.first_name,
                middle_name = excluded.middle_name,
                img_url = excluded.img_url,
                raw_data = excluded.raw_data,
                fr_id = coalesce(excluded.fr_id, eyefr.profiles.fr_id)
            "#,
        )
        .bind(profile.external_id.as_str())
        .bind(&profile.last_name)
        .bind(&profile.first_name)
        .bind(&profile.middle_name)
        .bind(&profile.image_url)
        .bind(&profile.raw_data)
        .bind(&profile.fr_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_profile_by_external_id(
        &self,
        external_id: &ExternalId,
    ) -> RepoResult<Option<ProfileRecord>> {
        let row = sqlx::query_as::<_, ProfileRow>(
            r#"
            select ext_id, first_name, last_name, middle_name, img_url, raw_data, fr_id
            from eyefr.profiles
            where ext_id = $1
            "#,
        )
        .bind(external_id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn get_profiles_by_external_ids(
        &self,
        external_ids: &[String],
    ) -> RepoResult<Vec<ProfileRecord>> {
        if external_ids.is_empty() {
            return Ok(vec![]);
        }

        let rows = sqlx::query_as::<_, ProfileRow>(
            r#"
            select ext_id, first_name, last_name, middle_name, img_url, raw_data, fr_id
            from eyefr.profiles
            where ext_id = any($1)
            "#,
        )
        .bind(external_ids)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect::<Result<Vec<_>, _>>()
    }

    pub async fn delete_profile_by_external_id(&self, external_id: &ExternalId) -> RepoResult<u64> {
        let res = sqlx::query(
            r#"
            delete from eyefr.profiles
            where ext_id = $1
            "#,
        )
        .bind(external_id.as_str())
        .execute(&self.pool)
        .await?;

        Ok(res.rows_affected())
    }

    pub async fn get_profile_by_fr_id(&self, fr_id: &str) -> RepoResult<Option<ProfileRecord>> {
        let row = sqlx::query_as::<_, ProfileRow>(
            r#"
            select ext_id, first_name, last_name, middle_name, img_url, raw_data, fr_id
            from eyefr.profiles
            where fr_id = $1
            "#,
        )
        .bind(fr_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn delete_profile_by_fr_id(&self, fr_id: &str) -> RepoResult<u64> {
        let res = sqlx::query(
            r#"
            delete from eyefr.profiles
            where fr_id = $1
            "#,
        )
        .bind(fr_id)
        .execute(&self.pool)
        .await?;

        Ok(res.rows_affected())
    }

    pub async fn search_profiles_by_last_name(
        &self,
        term: &str,
        limit: i64,
    ) -> RepoResult<Vec<ProfileRecord>> {
        let name_pattern = format!("{}%", term.trim());
        let rows = sqlx::query_as::<_, ProfileRow>(
            r#"
            select ext_id, first_name, last_name, middle_name, img_url, raw_data, fr_id
            from eyefr.profiles
            where last_name ilike $1
            order by last_name asc, first_name asc, ext_id asc
            limit $2
            "#,
        )
        .bind(name_pattern)
        .bind(normalized_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect::<Result<Vec<_>, _>>()
    }

    pub async fn find_profile_by_name(
        &self,
        first_name: &str,
        last_name: &str,
        middle_name: Option<&str>,
    ) -> RepoResult<Option<ProfileRecord>> {
        let middle_name = middle_name.map(str::trim).filter(|value| !value.is_empty());

        let row = sqlx::query_as::<_, ProfileRow>(
            r#"
            select ext_id, first_name, last_name, middle_name, img_url, raw_data, fr_id
            from eyefr.profiles
            where lower(coalesce(first_name, '')) = lower($1)
              and lower(coalesce(last_name, '')) = lower($2)
              and ($3::text is null or lower(coalesce(middle_name, '')) = lower($3))
            order by id desc
            limit 1
            "#,
        )
        .bind(first_name.trim())
        .bind(last_name.trim())
        .bind(middle_name)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn upsert_image(&self, image: &ImageRecord) -> RepoResult<()> {
        sqlx::query(
            r#"
            insert into eyefr.images (ext_id, data, size, url, quality, acceptability, raw_data)
            values ($1, $2, $3, $4, $5, $6, $7)
            on conflict (ext_id) do update
            set
                data = excluded.data,
                size = excluded.size,
                url = excluded.url,
                quality = excluded.quality,
                acceptability = excluded.acceptability,
                raw_data = excluded.raw_data,
                updated_at = now()
            "#,
        )
        .bind(image.external_id.as_str())
        .bind(&image.data)
        .bind(image.size)
        .bind(&image.url)
        .bind(image.quality)
        .bind(image.acceptability)
        .bind(&image.raw_data)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_image_by_external_id(
        &self,
        external_id: &ExternalId,
    ) -> RepoResult<Option<ImageRecord>> {
        let row = sqlx::query_as::<_, ImageRow>(
            r#"
            select ext_id, data, size, url, quality, acceptability, raw_data
            from eyefr.images
            where ext_id = $1
            "#,
        )
        .bind(external_id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn delete_image_by_external_id(&self, external_id: &ExternalId) -> RepoResult<u64> {
        let res = sqlx::query(
            r#"
            delete from eyefr.images
            where ext_id = $1
            "#,
        )
        .bind(external_id.as_str())
        .execute(&self.pool)
        .await?;

        Ok(res.rows_affected())
    }

    pub async fn insert_registration_error(
        &self,
        record: &RegistrationErrorRecord,
    ) -> RepoResult<()> {
        sqlx::query(
            r#"
            insert into eyefr.registration_errors (ext_id, fr_id, message)
            values ($1, $2, $3)
            "#,
        )
        .bind(record.external_id.as_ref().map(|id| id.as_str()))
        .bind(&record.fr_id)
        .bind(&record.message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_registration_errors_by_external_id(
        &self,
        external_id: &ExternalId,
        limit: i64,
    ) -> RepoResult<Vec<RegistrationErrorRecord>> {
        let rows = sqlx::query_as::<_, RegistrationErrorRow>(
            r#"
            select ext_id, fr_id, message
            from eyefr.registration_errors
            where ext_id = $1
            order by created_at desc
            limit $2
            "#,
        )
        .bind(external_id.as_str())
        .bind(normalized_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect::<Result<Vec<_>, _>>()
    }

    pub async fn append_enrollment_log(
        &self,
        code: &str,
        payload: &serde_json::Value,
    ) -> RepoResult<()> {
        if code.trim().is_empty() {
            return Err(RepoError::message("enrollment log code cannot be empty"));
        }

        sqlx::query(
            r#"
            insert into logs.enrollment (code, payload)
            values ($1, $2)
            "#,
        )
        .bind(code)
        .bind(payload)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_enrollment_logs_by_code(
        &self,
        code: &str,
        limit: i64,
    ) -> RepoResult<Vec<EnrollmentLogRecord>> {
        let rows = sqlx::query_as::<_, EnrollmentLogRow>(
            r#"
            select id, code, payload, retry_count
            from logs.enrollment
            where code = $1
            order by id desc
            limit $2
            "#,
        )
        .bind(code)
        .bind(normalized_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(EnrollmentLogRecord::from).collect())
    }

    pub async fn get_enrollment_logs(&self, limit: i64) -> RepoResult<Vec<EnrollmentLogRecord>> {
        let rows = sqlx::query_as::<_, EnrollmentLogRow>(
            r#"
            select id, code, payload, retry_count
            from logs.enrollment
            order by id desc
            limit $1
            "#,
        )
        .bind(normalized_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(EnrollmentLogRecord::from).collect())
    }

    pub async fn get_enrollment_roster(&self, limit: i64) -> RepoResult<Vec<ProfileRecord>> {
        let rows = sqlx::query_as::<_, ProfileRow>(
            r#"
            select ext_id, first_name, last_name, middle_name, img_url, raw_data, fr_id
            from eyefr.profiles
            order by last_name asc nulls last, first_name asc nulls last, ext_id asc
            limit $1
            "#,
        )
        .bind(normalized_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect::<Result<Vec<_>, _>>()
    }

    pub async fn get_enrollment_metadata(&self) -> RepoResult<EnrollmentMetadataRecord> {
        let row = sqlx::query_as::<_, EnrollmentMetadataRow>(
            r#"
            select
                (select count(*)::bigint from eyefr.profiles) as profiles_total,
                (select count(*)::bigint from eyefr.profiles where fr_id is not null and fr_id <> '') as profiles_with_fr_id,
                (select count(*)::bigint from eyefr.images) as images_total,
                (select count(*)::bigint from eyefr.registration_errors) as registration_errors_total,
                (select count(*)::bigint from logs.enrollment) as enrollment_logs_total
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(EnrollmentMetadataRecord::from(row))
    }

    pub async fn reset_enrollment_state(&self) -> RepoResult<EnrollmentResetRecord> {
        let mut tx = self.pool.begin().await?;

        let reg_res = sqlx::query(
            r#"
            delete from eyefr.registration_errors
            "#,
        )
        .execute(&mut *tx)
        .await?;

        let log_res = sqlx::query(
            r#"
            delete from logs.enrollment
            "#,
        )
        .execute(&mut *tx)
        .await?;

        let image_res = sqlx::query(
            r#"
            delete from eyefr.images
            "#,
        )
        .execute(&mut *tx)
        .await?;

        let profile_res = sqlx::query(
            r#"
            delete from eyefr.profiles
            "#,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(EnrollmentResetRecord {
            profiles_deleted: profile_res.rows_affected() as i64,
            images_deleted: image_res.rows_affected() as i64,
            registration_errors_deleted: reg_res.rows_affected() as i64,
            enrollment_logs_deleted: log_res.rows_affected() as i64,
        })
    }
}

#[derive(Debug, FromRow)]
struct ProfileRow {
    ext_id: String,
    first_name: Option<String>,
    last_name: Option<String>,
    middle_name: Option<String>,
    img_url: Option<String>,
    raw_data: Option<serde_json::Value>,
    fr_id: Option<String>,
}

#[derive(Debug, FromRow)]
struct ImageRow {
    ext_id: String,
    data: Vec<u8>,
    size: Option<f32>,
    url: Option<String>,
    quality: f32,
    acceptability: f32,
    raw_data: Option<serde_json::Value>,
}

#[derive(Debug, FromRow)]
struct RegistrationErrorRow {
    ext_id: Option<String>,
    fr_id: Option<String>,
    message: Option<String>,
}

#[derive(Debug, FromRow)]
struct EnrollmentLogRow {
    id: i64,
    code: String,
    payload: serde_json::Value,
    retry_count: Option<i32>,
}

#[derive(Debug, FromRow)]
struct EnrollmentMetadataRow {
    profiles_total: i64,
    profiles_with_fr_id: i64,
    images_total: i64,
    registration_errors_total: i64,
    enrollment_logs_total: i64,
}

impl TryFrom<ProfileRow> for ProfileRecord {
    type Error = RepoError;

    fn try_from(row: ProfileRow) -> Result<Self, Self::Error> {
        Ok(Self {
            external_id: ExternalId::try_from(row.ext_id)?,
            first_name: row.first_name,
            last_name: row.last_name,
            middle_name: row.middle_name,
            image_url: row.img_url,
            raw_data: row.raw_data,
            fr_id: row.fr_id,
        })
    }
}

impl TryFrom<ImageRow> for ImageRecord {
    type Error = RepoError;

    fn try_from(row: ImageRow) -> Result<Self, Self::Error> {
        Ok(Self {
            external_id: ExternalId::try_from(row.ext_id)?,
            data: row.data,
            size: row.size,
            url: row.url,
            quality: row.quality,
            acceptability: row.acceptability,
            raw_data: row.raw_data,
        })
    }
}

impl TryFrom<RegistrationErrorRow> for RegistrationErrorRecord {
    type Error = RepoError;

    fn try_from(row: RegistrationErrorRow) -> Result<Self, Self::Error> {
        let external_id = match row.ext_id {
            Some(ext_id) => Some(ExternalId::try_from(ext_id)?),
            None => None,
        };

        Ok(Self { external_id, fr_id: row.fr_id, message: row.message })
    }
}

impl From<EnrollmentLogRow> for EnrollmentLogRecord {
    fn from(row: EnrollmentLogRow) -> Self {
        Self { id: row.id, code: row.code, payload: row.payload, retry_count: row.retry_count }
    }
}

impl From<EnrollmentMetadataRow> for EnrollmentMetadataRecord {
    fn from(row: EnrollmentMetadataRow) -> Self {
        Self {
            profiles_total: row.profiles_total,
            profiles_with_fr_id: row.profiles_with_fr_id,
            images_total: row.images_total,
            registration_errors_total: row.registration_errors_total,
            enrollment_logs_total: row.enrollment_logs_total,
        }
    }
}

fn normalized_limit(limit: i64) -> i64 {
    if limit <= 0 {
        return 100;
    }

    limit.min(1000)
}

#[cfg(feature = "sqlx-typecheck")]
pub async fn typecheck_probe(pool: &PgPool) -> Result<(), sqlx::Error> {
    let _ = sqlx::query!(
        r#"
        select ext_id, fr_id
        from eyefr.profiles
        limit 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    let _ = sqlx::query!(
        r#"
        select ext_id
        from eyefr.profiles
        where fr_id is not null
        limit 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    let _ = sqlx::query!(
        r#"
        select ext_id, quality, acceptability
        from eyefr.images
        limit 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    let _ = sqlx::query!(
        r#"
        select ext_id, fr_id
        from eyefr.registration_errors
        limit 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    let _ = sqlx::query!(
        r#"
        select id, code, retry_count
        from logs.enrollment
        limit 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    let _ = sqlx::query!(
        r#"
        select
            (select count(*)::bigint from eyefr.profiles) as profiles_total,
            (select count(*)::bigint from eyefr.profiles where fr_id is not null and fr_id <> '') as profiles_with_fr_id,
            (select count(*)::bigint from eyefr.images) as images_total,
            (select count(*)::bigint from eyefr.registration_errors) as registration_errors_total,
            (select count(*)::bigint from logs.enrollment) as enrollment_logs_total
        "#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(())
}
