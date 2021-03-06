use js_sys::Promise;
use types::{model::ModelWithId, DeserializeOwned, Serialize};
use wasm_bindgen_futures::future_to_promise;

use crate::{fetcher::fetch, to_js};

pub fn by_id<M, V>(url: String, id: u64) -> Promise
where
    M: DeserializeOwned + Serialize + Send + ?Sized,
    V: Serialize,
    ModelWithId<M>: Into<V>,
{
    future_to_promise(async move {
        let model = fetch::<M>(url).await?;
        to_js(&ModelWithId { id, model }.into()).map_err(Into::into)
    })
}
