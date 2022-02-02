use curl::easy::Easy;
use std::collections::HashMap;

pub fn transfer(mut easy: Easy) -> Option<(u32, HashMap<String, String>, String)> {
    let mut headers: HashMap<String, String> = HashMap::with_capacity(25);
    let mut body: String = String::new();
    let transfer_result = {
        // Fetch data
        easy.useragent("kalkin/glv").unwrap();
        let mut transfer = easy.transfer();
        transfer
            .header_function(|line| {
                let line = String::from_utf8_lossy(line);
                let line = line.trim();
                let tmp: Vec<_> = line.splitn(2, ": ").collect();
                if tmp.len() == 2 {
                    let key = tmp[0].to_string();
                    let value = tmp[1].to_string();
                    headers.insert(key, value);
                }
                true
            })
            .unwrap();
        transfer
            .write_function(|data| {
                // body = String::from_utf8(Vec::from(data)).unwrap();
                body.push_str(String::from_utf8(Vec::from(data)).unwrap().as_str());
                Ok(data.len())
            })
            .unwrap();

        transfer.perform()
    };
    if let Err(e) = transfer_result {
        log::error!("{:?}", e);
        return None;
    }
    let response_code = easy.response_code().unwrap();
    Some((response_code, headers, body))
}
