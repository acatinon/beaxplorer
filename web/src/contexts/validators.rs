use std::convert::TryInto;

use db::models::ValidatorModel;
use serde::Serialize;
use types::EthSpec;

use crate::{contexts::common::breadcrumb::BreadcrumbPart, views::validator::ValidatorView};

use super::common::breadcrumb::Breadcrumb;

#[derive(Serialize)]
pub struct ValidatorsContext<E: EthSpec> {
    pub breadcrumb: Breadcrumb,
    pub validators: Vec<Option<ValidatorView<E>>>,
}

impl<E: EthSpec> ValidatorsContext<E> {
    pub fn new(validators: Vec<ValidatorModel>) -> Self {
        ValidatorsContext {
            breadcrumb: vec![BreadcrumbPart::from_text("Validators")].into(),
            validators: validators.into_iter().map(|e| e.try_into().ok()).collect(),
        }
    }
}