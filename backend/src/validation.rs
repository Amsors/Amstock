use crate::{
    error::{AppError, Result},
    models::{CreateElement, UpdateElement},
};

pub trait ElementInput {
    fn kind(&self) -> &str;
    fn tag_a(&self) -> &str;
    fn tag_b(&self) -> i64;
    fn tag_c(&self) -> i64;
    fn name(&self) -> &str;
    fn quantity(&self) -> f64;
}

macro_rules! impl_input {
    ($ty:ty) => {
        impl ElementInput for $ty {
            fn kind(&self) -> &str {
                &self.kind
            }
            fn tag_a(&self) -> &str {
                &self.tag_a
            }
            fn tag_b(&self) -> i64 {
                self.tag_b
            }
            fn tag_c(&self) -> i64 {
                self.tag_c
            }
            fn name(&self) -> &str {
                &self.name
            }
            fn quantity(&self) -> f64 {
                self.quantity
            }
        }
    };
}
impl_input!(CreateElement);
impl_input!(UpdateElement);

pub fn normalized_tag_a(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_uppercase();
    if value.len() == 1 && value.as_bytes()[0].is_ascii_alphabetic() {
        Ok(value)
    } else {
        Err(AppError::BadRequest("类别位 A 必须是一个英文字母".into()))
    }
}

pub fn validate_element(input: &impl ElementInput) -> Result<String> {
    if input.kind() != "item" && input.kind() != "container" {
        return Err(AppError::BadRequest(
            "元素类型必须是 item 或 container".into(),
        ));
    }
    if !(0..=99).contains(&input.tag_b()) || !(0..=99).contains(&input.tag_c()) {
        return Err(AppError::BadRequest(
            "助记位 BB 和 CC 必须在 00 到 99 之间".into(),
        ));
    }
    if input.name().trim().is_empty() {
        return Err(AppError::BadRequest("名称不能为空".into()));
    }
    if !input.quantity().is_finite() || input.quantity() < 0.0 {
        return Err(AppError::BadRequest("数量必须是非负有限数字".into()));
    }
    normalized_tag_a(input.tag_a())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CreateElement;

    fn valid() -> CreateElement {
        CreateElement {
            kind: "item".into(),
            tag_a: "m".into(),
            tag_b: 3,
            tag_c: 8,
            name: "螺母".into(),
            description: "M3 不锈钢".into(),
            quantity: 12.5,
            unit: "个".into(),
            parent_serial: None,
        }
    }

    #[test]
    fn normalizes_ascii_category_letter() {
        assert_eq!(validate_element(&valid()).unwrap(), "M");
    }

    #[test]
    fn rejects_invalid_tags_and_quantity() {
        let mut input = valid();
        input.tag_b = 100;
        assert!(validate_element(&input).is_err());
        input.tag_b = 1;
        input.quantity = f64::NAN;
        assert!(validate_element(&input).is_err());
        input.quantity = 1.0;
        input.tag_a = "中".into();
        assert!(validate_element(&input).is_err());
    }
}
