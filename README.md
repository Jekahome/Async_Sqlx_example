
## Basic example using async Sqlx

```shell
Install docker postgres
$ sudo docker pull postgres

$ docker run --name rust_job_queue  -e POSTGRES_USER=rust -e POSTGRES_PASSWORD=job_queue -p 5432:5432 postgres:latest
```

```shell
# restart docker job
$ docker start rust_job_queue

# migration
$ cargo run --bin migrations

# remove
$ docker rm -f rust_job_queue
```

## Links
[Sqlx async SQL](https://crates.io/crates/sqlx)

[Sqlx doc](https://docs.rs/sqlx/0.5.9/sqlx/index.html)

[Github Sqlx](https://github.com/launchbadge/sqlx)

[Sqlx Types postgres](https://github.com/launchbadge/sqlx/blob/be189bd11e6bdd14c45c70bdad477e780a82b050/sqlx-core/src/postgres/types/mod.rs)

[Все вопросы по теме sqlx](https://question-it.com/tags/sqlx)