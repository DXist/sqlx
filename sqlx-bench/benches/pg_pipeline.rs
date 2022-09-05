use criterion::{criterion_group, criterion_main, Criterion};
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgExtendedQueryPipeline, PgPool, PgPoolOptions};
use uuid::Uuid;

const MIGRATIONS_DIR: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");

struct Db(pub PgPool);
impl Db {
    fn setup() -> Self {
        let pool = Self::pool();

        let pool = sqlx_rt::block_on(async move {
            let m = Migrator::new(std::path::Path::new(MIGRATIONS_DIR)).await?;
            m.run(&pool).await?;
            Ok::<PgPool, sqlx::Error>(pool)
        })
        .expect("no errors");
        Self(pool)
    }

    fn pool() -> PgPool {
        sqlx_rt::block_on(
            PgPoolOptions::new()
                // only one connection is needed
                .min_connections(1)
                .max_connections(1)
                .connect(
                    &dotenvy::var("DATABASE_URL")
                        .expect("DATABASE_URL must be set to run benchmarks"),
                ),
        )
        .expect("failed to open PgPool")
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        sqlx_rt::block_on(async {
            let m = Migrator::new(std::path::Path::new(MIGRATIONS_DIR)).await?;
            m.undo(&self.0, 0).await
        })
        .expect("no errors");
    }
}

const EXPECTED_QUERIES_IN_PIPELINE: usize = 3;

fn construct_test_pipeline(
    user_id: Uuid,
    post_id: Uuid,
    comment_id: Uuid,
) -> PgExtendedQueryPipeline<'static, EXPECTED_QUERIES_IN_PIPELINE> {
    let user_insert_query = sqlx::query(
        "
        INSERT INTO \"user\" (user_id, username)
        VALUES
        ($1, $2)
    ",
    )
    .bind(user_id)
    .bind(format!("user {user_id}"));

    let mut pipeline =
        PgExtendedQueryPipeline::<EXPECTED_QUERIES_IN_PIPELINE>::from(user_insert_query);

    let post_insert_query = sqlx::query(
        "
        INSERT INTO post (post_id, user_id, content)
        VALUES
        ($1, $2, $3)
    ",
    )
    .bind(post_id)
    .bind(user_id)
    .bind("test post");

    pipeline.push(post_insert_query);

    let comment_insert_query = sqlx::query(
        "
        INSERT INTO comment (comment_id, post_id, user_id, content)
        VALUES
        ($1, $2, $3, $4)
    ",
    )
    .bind(comment_id)
    .bind(post_id)
    .bind(user_id)
    .bind("test comment");

    pipeline.push(comment_insert_query);
    pipeline
}

fn do_bench_pipeline(pool: &PgPool) {
    let user_id = Uuid::new_v4();
    let post_id = Uuid::new_v4();
    let comment_id = Uuid::new_v4();
    let pipeline = construct_test_pipeline(user_id, post_id, comment_id);
    let _ = sqlx_rt::block_on(async move { pipeline.execute(pool).await }).expect("no errors");
}

fn do_bench_single_queries(pool: &PgPool) {
    let user_id = Uuid::new_v4();
    let user_insert_query = sqlx::query(
        "
        INSERT INTO \"user\" (user_id, username)
        VALUES
        ($1, $2)
    ",
    )
    .bind(user_id)
    .bind(format!("user {user_id}"));

    let post_id = Uuid::new_v4();
    let post_insert_query = sqlx::query(
        "
        INSERT INTO post (post_id, user_id, content)
        VALUES
        ($1, $2, $3)
    ",
    )
    .bind(post_id)
    .bind(user_id)
    .bind("test post");

    let comment_id = Uuid::new_v4();
    let comment_insert_query = sqlx::query(
        "
        INSERT INTO comment (comment_id, post_id, user_id, content)
        VALUES
        ($1, $2, $3, $4)
    ",
    )
    .bind(comment_id)
    .bind(post_id)
    .bind(user_id)
    .bind("test comment");

    let _ = sqlx_rt::block_on(async {
        let mut tx = pool.begin().await?;
        user_insert_query.execute(&mut tx).await?;
        post_insert_query.execute(&mut tx).await?;
        comment_insert_query.execute(&mut tx).await?;
        tx.commit().await
    })
    .expect("no errors");
}

fn bench_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_pipeline");
    let db = Db::setup();

    group.bench_with_input(
        format!("user_post_comment_{EXPECTED_QUERIES_IN_PIPELINE}"),
        &db.0,
        |b, pool| {
            b.iter(|| do_bench_pipeline(pool));
        },
    );

    group.finish();
}

fn bench_multiple_inserts(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_multiple_inserts");
    let db = Db::setup();

    group.bench_with_input(format!("user_post_comment_3"), &db.0, |b, pool| {
        b.iter(|| do_bench_single_queries(pool));
    });

    group.finish();
}
criterion_group!(pg_pipeline, bench_pipeline);
criterion_group!(pg_multiple_inserts, bench_multiple_inserts);
criterion_main!(pg_pipeline, pg_multiple_inserts);
