use crate::params::IntoQueryParameters;
use crate::{
    backend::Backend, encode::Encode, error::Error, executor::Executor, params::QueryParameters,
    row::FromRow, types::HasSqlType, Row,
};
use bitflags::_core::marker::PhantomData;
use futures_core::{future::BoxFuture, stream::BoxStream};

pub struct Query<'q, DB, I = <DB as Backend>::QueryParameters, O = Row<DB>, T = O>
where
    DB: Backend,
{
    #[doc(hidden)]
    pub query: &'q str,

    #[doc(hidden)]
    pub input: I,

    #[doc(hidden)]
    pub output: PhantomData<O>,

    #[doc(hidden)]
    pub target: PhantomData<T>,

    #[doc(hidden)]
    pub backend: PhantomData<DB>,
}

impl<'q, DB, I: 'q, O: 'q> Query<'q, DB, I, O>
where
    DB: Backend,
    DB::QueryParameters: 'q,
    I: IntoQueryParameters<DB> + Send,
{
    #[inline]
    pub fn execute<E>(self, executor: &'q mut E) -> BoxFuture<'q, crate::Result<u64>>
    where
        E: Executor<Backend = DB>,
    {
        executor.execute(self.query, self.input)
    }
}

impl<'q, DB, I: 'q, O: 'q, T: 'q> Query<'q, DB, I, O, T>
where
    DB: Backend,
    DB::QueryParameters: 'q,
    I: IntoQueryParameters<DB> + Send,
    T: FromRow<DB, O> + Send + Unpin,
{
    pub fn fetch<E>(self, executor: &'q mut E) -> BoxStream<'q, crate::Result<T>>
    where
        E: Executor<Backend = DB>,
    {
        executor.fetch(self.query, self.input)
    }

    pub fn fetch_all<E>(self, executor: &'q mut E) -> BoxFuture<'q, crate::Result<Vec<T>>>
    where
        E: Executor<Backend = DB>,
    {
        executor.fetch_all(self.query, self.input)
    }

    pub fn fetch_optional<E>(self, executor: &'q mut E) -> BoxFuture<'q, Result<Option<T>, Error>>
    where
        E: Executor<Backend = DB>,
    {
        executor.fetch_optional(self.query, self.input)
    }

    pub fn fetch_one<E>(self, executor: &'q mut E) -> BoxFuture<'q, crate::Result<T>>
    where
        E: Executor<Backend = DB>,
    {
        executor.fetch_one(self.query, self.input)
    }
}

impl<DB, Target> Query<'_, DB, <DB as Backend>::QueryParameters, Row<DB>, Target>
where
    DB: Backend,
{
    /// Bind a value for use with this SQL query.
    ///
    /// # Safety
    ///
    /// This function should be used with care, as SQLx cannot validate
    /// that the value is of the right type nor can it validate that you have
    /// passed the correct number of parameters.
    pub fn bind<T>(mut self, value: T) -> Self
    where
        DB: HasSqlType<T>,
        T: Encode<DB>,
    {
        self.input.bind(value);
        self
    }
}

/// Construct a full SQL query using raw SQL.
#[inline]
pub fn query<DB, T>(query: &str) -> Query<'_, DB, DB::QueryParameters, Row<DB>, T>
where
    DB: Backend,
{
    Query {
        query,
        input: DB::QueryParameters::new(),
        output: PhantomData,
        backend: PhantomData,
        target: PhantomData,
    }
}
