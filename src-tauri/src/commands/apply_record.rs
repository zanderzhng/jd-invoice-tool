use crate::api::jd::JdApi;
use crate::utils::file::read_cookie_file;
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRecordProduct {
    pub name: String,
    pub sku_name: String,
    pub img_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRecord {
    pub order_id: String,
    pub status: String,
    pub req_state: i64,
    pub req_type: i64,
    pub apply_time: String,
    pub invoice_title: String,
    pub invoice_type: String,
    pub invoice_content_name: String,
    pub title_type: String,
    pub amount: String,
    pub vender_name: String,
    pub tag_str: String,
    pub products: Vec<ApplyRecordProduct>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRecordDetail {
    pub order_id: String,
    pub status: String,
    pub invoice_title: String,
    pub invoice_type: String,
    pub invoice_content_name: String,
    pub title_type: String,
    pub invoice_amount: String,
    pub invoice_time: String,
    pub apply_time: String,
    pub vender_name: String,
    pub files: Vec<crate::api::jd::InvoiceFile>,
}

fn get_string(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(text)) => text.trim().to_string(),
        Some(serde_json::Value::Number(number)) => number.to_string(),
        Some(serde_json::Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn get_i64(value: Option<&serde_json::Value>, default: i64) -> i64 {
    value
        .and_then(|item| item.as_i64().or_else(|| item.as_str().and_then(|text| text.parse::<i64>().ok())))
        .unwrap_or(default)
}

fn format_amount(value: &str) -> String {
    value
        .parse::<f64>()
        .map(|amount| format!("{:.2}", amount))
        .unwrap_or_else(|_| value.to_string())
}

fn invoice_type_label(value: &str) -> String {
    match value.parse::<i64>().unwrap_or_default() {
        1 => "纸质普票".to_string(),
        2 => "专用发票".to_string(),
        3 => "电子普通发票".to_string(),
        22 => "电子专用发票".to_string(),
        23 => "数电普票".to_string(),
        24 => "数电专票".to_string(),
        _ => value.to_string(),
    }
}

fn title_type_label(value: &str) -> String {
    match value.parse::<i64>().unwrap_or_default() {
        4 => "个人".to_string(),
        5 => "单位".to_string(),
        _ => value.to_string(),
    }
}

fn status_label(req_state: i64, req_type: i64) -> String {
    match req_state {
        2 | 3 => {
            if req_type == 2 {
                "已换开".to_string()
            } else {
                "已补开".to_string()
            }
        }
        4 => "已驳回".to_string(),
        0 | 1 | 5 => {
            if req_type == 2 {
                "换开中".to_string()
            } else {
                "申请中".to_string()
            }
        }
        _ => "未知".to_string(),
    }
}

fn parse_products(order: &serde_json::Value) -> Vec<ApplyRecordProduct> {
    order
        .get("products")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|item| ApplyRecordProduct {
            name: get_string(item.get("name")),
            sku_name: get_string(item.get("skuName")),
            img_url: get_string(item.get("imgUrl")),
        })
        .collect()
}

fn parse_center_detail(data: &serde_json::Value, order_id: &str, files: Vec<crate::api::jd::InvoiceFile>) -> ApplyRecordDetail {
    let center_detail = data
        .get("centerDetailList")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let invoice_req = data.get("invoiceReq").cloned().unwrap_or_else(|| serde_json::json!({}));
    let invoice_state = data.get("invoiceState").cloned().unwrap_or_else(|| serde_json::json!({}));

    let invoice_type = get_string(center_detail.get("ivcType")).if_empty_then(|| get_string(invoice_req.get("ivcType")));
    let title_type = get_string(center_detail.get("ivcTitleType")).if_empty_then(|| get_string(invoice_req.get("ivcTitleType")));

    ApplyRecordDetail {
        order_id: order_id.to_string(),
        status: get_string(invoice_state.get("stateText")),
        invoice_title: get_string(center_detail.get("ivcTitle")).if_empty_then(|| get_string(invoice_req.get("ivcTitle"))),
        invoice_type: invoice_type_label(&invoice_type),
        invoice_content_name: get_string(center_detail.get("ivcContentName")),
        title_type: title_type_label(&title_type),
        invoice_amount: get_string(center_detail.get("invoiceAmount")).if_empty_then(|| get_string(invoice_state.get("price"))),
        invoice_time: get_string(center_detail.get("invoiceTimeByZone")).if_empty_then(|| get_string(center_detail.get("invoiceTime"))),
        apply_time: get_string(center_detail.get("applyTimeByZone")).if_empty_then(|| get_string(invoice_req.get("createTime"))),
        vender_name: get_string(data.get("venderName")),
        files,
    }
}

trait EmptyStringExt {
    fn if_empty_then<F: FnOnce() -> String>(self, fallback: F) -> String;
}

impl EmptyStringExt for String {
    fn if_empty_then<F: FnOnce() -> String>(self, fallback: F) -> String {
        if self.is_empty() {
            fallback()
        } else {
            self
        }
    }
}

#[command]
pub async fn fetch_apply_records(page: u32) -> Result<Vec<ApplyRecord>, String> {
    let cookie = read_cookie_file()?.ok_or("Cookie not found, please login first")?;
    let api = JdApi::new(cookie);
    let orders = api.get_apply_list(page).await?;

    Ok(orders
        .into_iter()
        .map(|order| {
            let req_state = get_i64(order.get("reqState"), -1);
            let req_type = get_i64(order.get("reqType"), 0);
            let ivc_type = get_string(order.get("ivcType"));
            let title_type = get_string(order.get("ivcTitleType"));

            ApplyRecord {
                order_id: get_string(order.get("orderId")),
                status: status_label(req_state, req_type),
                req_state,
                req_type,
                apply_time: get_string(order.get("reqTimeByZone")).if_empty_then(|| get_string(order.get("reqTime"))),
                invoice_title: get_string(order.get("ivcTitle")),
                invoice_type: invoice_type_label(&ivc_type),
                invoice_content_name: get_string(order.get("ivcContentName")),
                title_type: title_type_label(&title_type),
                amount: format_amount(&get_string(order.get("ivcAmount"))),
                vender_name: get_string(order.get("venderName")),
                tag_str: get_string(order.get("tagStr")),
                products: parse_products(&order),
            }
        })
        .collect())
}

#[command]
pub async fn fetch_apply_record_detail(order_id: String, tag_str: String) -> Result<ApplyRecordDetail, String> {
    let cookie = read_cookie_file()?.ok_or("Cookie not found, please login first")?;
    let api = JdApi::new(cookie);
    let data = api.get_invoice_center_detail(&order_id, &tag_str).await?;
    let files = api.get_invoice_detail(&order_id).await.unwrap_or_default();

    Ok(parse_center_detail(&data, &order_id, files))
}
