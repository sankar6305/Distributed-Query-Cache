use flatbuffers::FlatBufferBuilder;

use crate::db::QueryResult;
use crate::schema::{ResultRow, ResultRowArgs, ResultSet, ResultSetArgs};

/// Encodes a query result as a FlatBuffers `ResultSet` — this is what gets
/// stored in Redis and returned in `QueryResponse.result`. Reading it back
/// (see `decode`) never has to walk/copy the whole buffer up front, unlike a
/// protobuf/JSON decode.
pub fn encode(result: &QueryResult) -> Vec<u8> {
    let mut fbb = FlatBufferBuilder::new();

    let columns: Vec<_> = result
        .columns
        .iter()
        .map(|c| fbb.create_string(c))
        .collect();
    let columns_vec = fbb.create_vector(&columns);

    let rows: Vec<_> = result
        .rows
        .iter()
        .map(|row| {
            let values: Vec<_> = row.iter().map(|v| fbb.create_string(v)).collect();
            let values_vec = fbb.create_vector(&values);
            ResultRow::create(
                &mut fbb,
                &ResultRowArgs {
                    values: Some(values_vec),
                },
            )
        })
        .collect();
    let rows_vec = fbb.create_vector(&rows);

    let result_set = ResultSet::create(
        &mut fbb,
        &ResultSetArgs {
            columns: Some(columns_vec),
            rows: Some(rows_vec),
        },
    );
    fbb.finish(result_set, None);
    fbb.finished_data().to_vec()
}

#[derive(Debug, PartialEq, Eq)]
pub struct DecodedResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn decode(bytes: &[u8]) -> Result<DecodedResult, String> {
    let result_set = flatbuffers::root::<ResultSet>(bytes).map_err(|e| e.to_string())?;

    let columns = result_set
        .columns()
        .map(|v| v.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let rows = result_set
        .rows()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.values()
                        .map(|vals| vals.iter().map(|s| s.to_string()).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(DecodedResult { columns, rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_columns_and_rows() {
        let result = QueryResult {
            columns: vec!["id".into(), "amount".into()],
            rows: vec![
                vec!["1".into(), "42.5".into()],
                vec!["2".into(), "7".into()],
            ],
        };

        let bytes = encode(&result);
        let decoded = decode(&bytes).unwrap();

        assert_eq!(decoded.columns, result.columns);
        assert_eq!(decoded.rows, result.rows);
    }

    #[test]
    fn round_trips_empty_result() {
        let result = QueryResult {
            columns: vec!["only_col".into()],
            rows: vec![],
        };

        let bytes = encode(&result);
        let decoded = decode(&bytes).unwrap();

        assert_eq!(decoded.columns, result.columns);
        assert!(decoded.rows.is_empty());
    }
}
