/* use serde::Deserialize;

const KEY: &str = "X-Api-Key";

#[derive(Deserialize, Debug)]
struct CursorIn {
    b: String,
    bool: String,
    bs: String,
    l: String,
    m: String,
    n: String,
    ns: String,
    null: String,
    s: String,
    ss: String,
}

#[derive(Deserialize, Debug)]
struct Cursor {
    hash: CursorIn,
    point_in_time: CursorIn,
}

#[derive(Deserialize, Debug)]
struct BlockDescriptor {
    id: String,
    timestamp: u64,
    #[serde(rename(deserialize = "totalFee"))]
    total_fee: u64,
    #[serde(rename(deserialize = "operationCount"))]
    operation_count: u64,
    #[serde(rename(deserialize = "creatorAddress"))]
    creator_address: String,
}

#[derive(Deserialize, Debug)]
struct Blocks {
    records: Vec<BlockDescriptor>,
    #[serde(rename(deserialize = "nextCursor"))]
    next_cursor: Cursor,
}

pub(crate) fn fetch_block_from_node_storer() -> Result<(), reqwest::Error> {
    let url = "https://ikfuju74kk.execute-api.eu-west-3.amazonaws.com/Prod/".to_string()
    // + "/operation/O1a86XqERaR5MSuLx4HebYt4f2U5jJfDW7p4JMobL9714xGMsZp";
    + "/block";
    let x_api_key = "3epwEGrAlb2MnQwAK3hFfonYJAXXwUx5Z7dHFmp3";

    let client = reqwest::blocking::Client::new();
    let res = client.get(&url).header(KEY, x_api_key).send().unwrap();
    let value: Blocks = res.json().unwrap();
    println!("################################################################################");
    println!("body = {:?}", value);
    println!("################################################################################");

    Ok(())
}
 */