use std::convert::Infallible;

use model::Flow;

datacache::storage_ref!(pub StorageRef);
// ($vis:vis $ident:ident($exc:ty, $data:ty), id($id_field:ident: $id_ty:ty), unique($($unique:ident: $unique_ty:ty),* ), fields($($field:ident: $field_ty:ty),* )) => {

pub struct FlowExecutor {}
impl datacache::DataQueryExecutor<Flow> for FlowExecutor {
    type Error = Infallible;

    type Id = i32;

    fn find_one<'life0, 'async_trait>(
        &'life0 self,
        query: <Flow as datacache::DataMarker>::Query,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Flow, Self::Error>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    fn find_all_ids<'life0, 'async_trait>(
        &'life0 self,
        query: <Flow as datacache::DataMarker>::Query,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Vec<Self::Id>, Self::Error>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    fn find_optional<'life0, 'async_trait>(
        &'life0 self,
        query: <Flow as datacache::DataMarker>::Query,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Option<Flow>, Self::Error>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    fn create<'life0, 'async_trait>(
        &'life0 self,
        data: datacache::Data<Flow>,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<(), Self::Error>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    fn update<'life0, 'async_trait>(
        &'life0 self,
        data: datacache::Data<Flow>,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<(), Self::Error>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    fn delete<'life0, 'async_trait>(
        &'life0 self,
        data: <Flow as datacache::DataMarker>::Query,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Vec<Self::Id>, Self::Error>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }
}

datacache::storage!(pub FlowStorage(FlowExecutor, Flow), id(uid: i32), unique(slug: String), fields());
datacache::storage_manager!(pub StorageManager: StorageRef);
