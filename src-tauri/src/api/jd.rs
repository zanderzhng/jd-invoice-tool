use aes::Aes128;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use cbc::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
use serde::{Deserialize, Serialize};
use crate::api::client::build_client;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

type Aes128CbcEnc = cbc::Encryptor<Aes128>;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceFile {
    #[serde(rename = "ivcNo", default)]
    pub ivc_no: String,
    #[serde(default)]
    pub amount: String,
    #[serde(rename = "invoiceDate", default)]
    pub invoice_date: String,
    #[serde(rename = "fileUrl", default)]
    pub file_url: String,
    #[serde(rename = "pdfUrl", default)]
    pub pdf_url: String,
    #[serde(rename = "imgUrl", default)]
    pub img_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MergeGroupItem {
    #[serde(rename = "orgName")]
    pub org_name: Option<String>,
    #[serde(rename = "orderList")]
    pub order_list: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchOrderSeed {
    pub order_id: String,
    pub order_amount_query: String,
    pub resolved_amount: Option<String>,
    pub original_order_info: serde_json::Value,
}

fn parse_data_field(json: &serde_json::Value) -> Option<serde_json::Value> {
    let data = json.get("data")?;
    if let Some(s) = data.as_str() {
        serde_json::from_str(s).ok()
    } else {
        Some(data.clone())
    }
}

fn extract_orders(data: &serde_json::Value) -> Vec<serde_json::Value> {
    // 尝试 data.orders
    if let Some(arr) = data.get("orders").and_then(|o| o.as_array()) {
        return arr.clone();
    }
    // 尝试 data.data.orders
    if let Some(inner) = data.get("data") {
        if let Some(arr) = inner.get("orders").and_then(|o| o.as_array()) {
            return arr.clone();
        }
        // 尝试 data.data 是数组
        if let Some(arr) = inner.as_array() {
            return arr.clone();
        }
    }
    // 尝试 data 本身就是数组
    if let Some(arr) = data.as_array() {
        return arr.clone();
    }
    Vec::new()
}

fn parse_json_array(value: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    match value {
        Some(v) => {
            if let Some(arr) = v.as_array() {
                arr.clone()
            } else if let Some(s) = v.as_str() {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
                    parsed.as_array().cloned().unwrap_or_default()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        }
        None => Vec::new(),
    }
}

fn can_exchange_invoice(order: &serde_json::Value) -> bool {
    order
        .get("canHk")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn is_success_code(json: &serde_json::Value) -> bool {
    match json.get("code") {
        Some(serde_json::Value::Number(value)) => value.as_i64() == Some(0),
        Some(serde_json::Value::String(value)) => value == "0",
        _ => false,
    }
}

fn extract_message(json: &serde_json::Value) -> String {
    json.get("msg")
        .and_then(|value| value.as_str())
        .unwrap_or("Request failed")
        .to_string()
}

pub fn encrypt_aes_cbc_base64(plain_text: &str, key: &str, iv: &str) -> Result<String, String> {
    if key.as_bytes().len() != 16 || iv.as_bytes().len() != 16 {
        return Err("Invalid pressKey length returned by JD".to_string());
    }

    let mut buffer = plain_text.as_bytes().to_vec();
    let message_len = buffer.len();
    let block_size = 16;
    let padding = block_size - (message_len % block_size);
    buffer.resize(message_len + padding, 0);

    let encrypted = Aes128CbcEnc::new_from_slices(key.as_bytes(), iv.as_bytes())
        .map_err(|e| format!("Failed to init AES cipher: {}", e))?
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, message_len)
        .map_err(|e| format!("Failed to encrypt payload: {}", e))?;

    Ok(STANDARD.encode(encrypted))
}

pub struct JdApi {
    cookie: String,
}

impl JdApi {
    pub fn new(cookie: String) -> Self {
        Self { cookie }
    }

    pub async fn get_invoice_list(&self, page: u32) -> Result<Vec<serde_json::Value>, String> {
        let client = build_client(&self.cookie)?;
        let url = format!(
            "https://api.m.jd.com/api?functionId=appFpzz_getAllNextOrderPage&appid=invoice-m&xAPIClientLanguage=zh_CN&xAPICurrency=CNY&xAPIRegion=CN&xAPITz=Asia%2FShanghai&xAPIElder=0&page={}",
            page
        );

        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        let data = parse_data_field(&json).unwrap_or_default();
        let orders = extract_orders(&data);

        Ok(orders)
    }

    pub async fn get_orders_amount(&self, pass_keys: &[String]) -> Result<serde_json::Value, String> {
        let client = build_client(&self.cookie)?;
        let order_ids = pass_keys.join(";");
        let encoded_ids = utf8_percent_encode(&order_ids, NON_ALPHANUMERIC).to_string();
        let url = format!(
            "https://myivc.jd.com/newIvc/appFpzz/getAppOrdersAmountAsync.action?orderIds={}",
            encoded_ids
        );

        eprintln!("[DEBUG] get_orders_amount pass_keys count={}", pass_keys.len());
        eprintln!("[DEBUG] get_orders_amount url: {}", url);

        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        eprintln!("[DEBUG] get_orders_amount raw response: {}", serde_json::to_string(&json).unwrap_or_default());

        Ok(json)
    }

    pub async fn get_invoice_detail(&self, order_id: &str) -> Result<Vec<InvoiceFile>, String> {
        let client = build_client(&self.cookie)?;
        let url = format!(
            "https://myivc.jd.com/newIvc/appFpzz/appPreviewInvoice.action?orderId={}&canCard=0&canShare=true",
            order_id
        );

        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        eprintln!("[DEBUG] get_invoice_detail orderId={} raw response: {}", order_id, serde_json::to_string(&json).unwrap_or_default());

        let data = parse_data_field(&json).unwrap_or(serde_json::Value::Null);

        let file_list = parse_json_array(data.get("fileUrlList"));

        eprintln!("[DEBUG] get_invoice_detail orderId={} fileUrlList count={}", order_id, file_list.len());

        let files = file_list
            .iter()
            .filter_map(|v| serde_json::from_value::<InvoiceFile>(v.clone()).ok())
            .collect::<Vec<_>>();

        Ok(files)
    }

    pub async fn get_apply_list(&self, page: u32) -> Result<Vec<serde_json::Value>, String> {
        let client = build_client(&self.cookie)?;
        let url = format!(
            "https://api.m.jd.com/api?functionId=appFpzz_getApplyNextOrderPage&appid=invoice-m&xAPIClientLanguage=zh_CN&xAPICurrency=CNY&xAPIRegion=CN&xAPITz=Asia%2FShanghai&xAPIElder=0&page={}",
            page
        );

        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        let data = parse_data_field(&json).unwrap_or_default();
        let orders = extract_orders(&data);
        eprintln!("[DEBUG] get_apply_list page={} orders count={}", page, orders.len());

        Ok(orders)
    }

    pub async fn get_invoice_center_detail(
        &self,
        order_id: &str,
        tag_str: &str,
    ) -> Result<serde_json::Value, String> {
        let client = build_client(&self.cookie)?;
        let encoded_tag = utf8_percent_encode(tag_str, NON_ALPHANUMERIC).to_string();
        let url = format!(
            "https://api.m.jd.com/api?functionId=appFpzz_appIvcCenterDetail&appid=invoice-m&xAPIClientLanguage=zh_CN&xAPICurrency=CNY&xAPIRegion=CN&xAPITz=Asia%2FShanghai&xAPIElder=0&orderId={}&tagStr={}",
            order_id, encoded_tag
        );

        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        if !is_success_code(&json) {
            return Err(extract_message(&json));
        }

        let data = parse_data_field(&json).unwrap_or(serde_json::Value::Null);
        eprintln!("[DEBUG] get_invoice_center_detail orderId={} success={}", order_id, !data.is_null());
        Ok(data)
    }

    pub async fn get_batch_list(&self, page: u32) -> Result<Vec<serde_json::Value>, String> {
        let client = build_client(&self.cookie)?;
        let url = format!(
            "https://myivc.jd.com/newIvc/appFpzz/getBatchNextOrderPage.action?page={}",
            page
        );

        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        eprintln!("[DEBUG] get_batch_list page={} raw response: {}", page, serde_json::to_string(&json).unwrap_or_default());

        let data = parse_data_field(&json).unwrap_or_default();
        let orders = extract_orders(&data);

        eprintln!("[DEBUG] get_batch_list page={} orders count={}", page, orders.len());

        Ok(orders)
    }

    async fn resolve_page_amounts(&self, page: u32, seeds: &mut [BatchOrderSeed]) -> Result<(), String> {
        const AMOUNT_BATCH_SIZE: usize = 5;

        let queries: Vec<String> = seeds
            .iter()
            .map(|seed| seed.order_amount_query.clone())
            .filter(|query| !query.is_empty())
            .collect();

        if queries.is_empty() {
            eprintln!("[DEBUG] resolve_page_amounts page={} skipped: no valid orderAmountQuery", page);
            return Ok(());
        }

        let mut amount_map = std::collections::HashMap::new();

        for (chunk_index, chunk) in queries.chunks(AMOUNT_BATCH_SIZE).enumerate() {
            eprintln!(
                "[DEBUG] resolve_page_amounts page={} chunk={} chunk_size={}",
                page,
                chunk_index + 1,
                chunk.len()
            );

            let json = self.get_orders_amount(chunk).await?;
            let data = parse_data_field(&json).unwrap_or(serde_json::Value::Null);
            let items = data.as_array().cloned().unwrap_or_default();

            eprintln!(
                "[DEBUG] resolve_page_amounts page={} chunk={} matched={}",
                page,
                chunk_index + 1,
                items.len()
            );

            for item in items {
                let order_id = item
                    .get("orderId")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string();
                let amount = item
                    .get("money")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string();

                if !order_id.is_empty() && !amount.is_empty() {
                    amount_map.insert(order_id, amount);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }

        let mut unresolved = Vec::new();
        for seed in seeds.iter_mut() {
            seed.resolved_amount = amount_map.get(&seed.order_id).cloned();
            if seed.resolved_amount.is_none() {
                unresolved.push(seed.order_id.clone());
            }
        }

        eprintln!(
            "[DEBUG] resolve_page_amounts page={} total_candidates={} resolved={} unresolved={}",
            page,
            seeds.len(),
            amount_map.len(),
            unresolved.len()
        );
        if !unresolved.is_empty() {
            eprintln!("[DEBUG] resolve_page_amounts page={} unresolved orderIds={:?}", page, unresolved);
        }

        Ok(())
    }

    pub async fn get_all_batch_orders(&self) -> Result<Vec<BatchOrderSeed>, String> {
        let mut all = Vec::new();
        let mut page = 1;

        loop {
            let list = self.get_batch_list(page).await?;
            if list.is_empty() {
                break;
            }
            let mut page_seeds = Vec::new();
            for order in list {
                if !can_exchange_invoice(&order) {
                    let order_id = order
                        .get("orderId")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    let reason = order
                        .get("canHkReason")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    eprintln!(
                        "[DEBUG] get_all_batch_orders skipped order by canHk=false: {} reason={}",
                        order_id,
                        reason
                    );
                    continue;
                }
                if let Some(info) = order.get("originalOrderInfo").cloned() {
                    page_seeds.push(BatchOrderSeed {
                        order_id: order
                            .get("orderId")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        order_amount_query: order
                            .get("orderAmountQuery")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        resolved_amount: None,
                        original_order_info: info,
                    });
                }
            }

            self.resolve_page_amounts(page, &mut page_seeds).await?;
            all.extend(page_seeds);
            page += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
        }

        eprintln!("[DEBUG] get_all_batch_orders total={}", all.len());

        Ok(all)
    }

    pub async fn get_batch_bk_orders(&self, orders_json: &str) -> Result<serde_json::Value, String> {
        let client = build_client(&self.cookie)?;

        let res = client
            .post("https://myivc.jd.com/newIvc/appFpzz/appBatchHkfpReq.action")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!(
                "orders={}",
                utf8_percent_encode(orders_json, NON_ALPHANUMERIC)
            ))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        eprintln!(
            "[DEBUG] get_batch_bk_orders raw response: {}",
            serde_json::to_string(&json).unwrap_or_default()
        );

        if !is_success_code(&json) {
            return Err(extract_message(&json));
        }

        Ok(json)
    }

    pub async fn get_app_batch_can_vat_async(
        &self,
        order_ids: &str,
    ) -> Result<serde_json::Value, String> {
        let client = build_client(&self.cookie)?;
        let url = "https://api.m.jd.com/api?functionId=appFpzz_getAppBatchCanVatAsync&appid=invoice-m&xAPIClientLanguage=zh_CN&xAPICurrency=CNY&xAPIRegion=CN&xAPITz=Asia%2FShanghai&xAPIElder=0";

        let res = client
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!(
                "orderIds={}",
                utf8_percent_encode(order_ids, NON_ALPHANUMERIC)
            ))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        eprintln!(
            "[DEBUG] get_app_batch_can_vat_async raw response: {}",
            serde_json::to_string(&json).unwrap_or_default()
        );

        if !is_success_code(&json) {
            return Err(extract_message(&json));
        }

        Ok(json)
    }

    pub async fn check_merge_hkfp_req(
        &self,
        batch_self_invoice_req_json: &str,
        user_order_list_json: &str,
    ) -> Result<serde_json::Value, String> {
        let client = build_client(&self.cookie)?;
        let body = format!(
            "batchSelfInvoiceReqJson={}&userOrderListJson={}",
            utf8_percent_encode(batch_self_invoice_req_json, NON_ALPHANUMERIC),
            utf8_percent_encode(user_order_list_json, NON_ALPHANUMERIC)
        );

        let res = client
            .post("https://myivc.jd.com/newIvc/appFpzz/checkMergeHkfpReq.action")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        eprintln!(
            "[DEBUG] check_merge_hkfp_req raw response: {}",
            serde_json::to_string(&json).unwrap_or_default()
        );

        if !is_success_code(&json) {
            return Err(extract_message(&json));
        }

        Ok(json)
    }

    pub async fn app_do_merge_hkfp_req(
        &self,
        batch_self_invoice_req_json: &str,
        merge_group_list_json: &str,
    ) -> Result<serde_json::Value, String> {
        let client = build_client(&self.cookie)?;
        let body = format!(
            "batchSelfInvoiceReqJson={}&mergeGroupListJson={}",
            utf8_percent_encode(batch_self_invoice_req_json, NON_ALPHANUMERIC),
            utf8_percent_encode(merge_group_list_json, NON_ALPHANUMERIC)
        );

        let res = client
            .post("https://myivc.jd.com/newIvc/appFpzz/appDoMergeHkfpReq")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        eprintln!(
            "[DEBUG] app_do_merge_hkfp_req raw response: {}",
            serde_json::to_string(&json).unwrap_or_default()
        );

        if !is_success_code(&json) {
            return Err(extract_message(&json));
        }

        Ok(json)
    }

    pub async fn check_merge(&self, user_order_list_json: &str) -> Result<Vec<MergeGroupItem>, String> {
        let client = build_client(&self.cookie)?;
        let body = format!(
            "userOrderListJson={}&batchSelfInvoiceReqJson=BOWblSt7NzlDfq1MuE/ixaG3nBRJvhgNyOwR95Du8dqkpzFZdqKOGiJL/mv88dm19KqPeZ42UbP2AXqcXG0v0Kt/eE3P9bU05eI0qRx1PRL06PFKgQPZH66aV1lFPrjRpWVQVgtZRIS4XQ7FhqdvPC/tV7/0CUSaKBKEnzdFr63D7UiQyRBXaEFO2B8RJERSjxUXsjfxFLXvF/oBvDoGRu5hUprZJa6SOHMbPwiPPzCeFWAPCM3Fi0hZPsDMfzqDL/maWsxtvE01e1jBPmpLTA==",
            user_order_list_json
        );

        let res = client
            .post("https://myivc.jd.com/newIvc/appFpzz/checkMergeHkfpReq.action")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Parse JSON failed: {}", e))?;

        eprintln!("[DEBUG] check_merge raw response: {}", serde_json::to_string(&json).unwrap_or_default());

        let data = parse_data_field(&json).unwrap_or(serde_json::Value::Null);

        let group_list = parse_json_array(data.get("groupList"));

        eprintln!("[DEBUG] check_merge groupList count={}", group_list.len());

        let groups = group_list
            .iter()
            .filter_map(|v| serde_json::from_value::<MergeGroupItem>(v.clone()).ok())
            .collect::<Vec<_>>();

        Ok(groups)
    }
}
