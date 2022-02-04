/*
 * Copyright 2021 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use boolinator::Boolinator;

use crate::error::ServiceError;
use crate::misc::check_weight_peer_id;
use crate::record::Record;
use crate::record_storage_impl::merge_records;
use crate::results::{
    DhtResult, GetValuesResult, MergeResult, PutHostRecordResult, RepublishValuesResult,
};
use crate::storage_impl::get_storage;
use crate::tetraplets_checkers::{
    check_host_value_tetraplets, check_timestamp_tetraplets, check_weight_tetraplets,
};
use crate::{wrapped_try, WeightResult};
use marine_rs_sdk::marine;

#[marine]
pub fn get_record_bytes(
    key_id: String,
    value: String,
    relay_id: Vec<String>,
    service_id: Vec<String>,
    timestamp_created: u64,
) -> Vec<u8> {
    let cp = marine_rs_sdk::get_call_parameters();
    Record::signature_bytes(
        key_id,
        value,
        cp.init_peer_id.clone(),
        cp.init_peer_id,
        relay_id,
        service_id,
        timestamp_created,
    )
}

#[marine]
pub fn put_record(
    key_id: String,
    value: String,
    relay_id: Vec<String>,
    service_id: Vec<String>,
    timestamp_created: u64,
    signature: Vec<u8>,
    weight: WeightResult,
    current_timestamp_sec: u64,
) -> DhtResult {
    wrapped_try(|| {
        let cp = marine_rs_sdk::get_call_parameters();
        check_weight_tetraplets(&cp, 6)?;
        check_timestamp_tetraplets(&cp, 7)?;
        check_weight_peer_id(&cp.init_peer_id, &weight)?;
        let record = Record {
            key_id,
            value,
            peer_id: cp.init_peer_id.clone(),
            set_by: cp.init_peer_id,
            relay_id,
            service_id,
            timestamp_created,
            signature,
            weight: weight.weight,
        };
        record.verify(current_timestamp_sec)?;

        let storage = get_storage()?;
        storage.check_key_existence(&record.key_id)?;
        storage.update_record(record, false).map(|_| {})
    })
    .into()
}

#[marine]
pub fn get_host_record_bytes(
    key_id: String,
    value: String,
    relay_id: Vec<String>,
    service_id: Vec<String>,
    timestamp_created: u64,
) -> Vec<u8> {
    let cp = marine_rs_sdk::get_call_parameters();
    Record::signature_bytes(
        key_id,
        value,
        cp.host_id,
        cp.init_peer_id,
        relay_id,
        service_id,
        timestamp_created,
    )
}
#[marine]
pub fn put_host_record(
    key_id: String,
    value: String,
    relay_id: Vec<String>,
    service_id: Vec<String>,
    timestamp_created: u64,
    signature: Vec<u8>,
    weight: WeightResult,
    current_timestamp_sec: u64,
) -> PutHostRecordResult {
    wrapped_try(|| {
        let cp = marine_rs_sdk::get_call_parameters();
        check_weight_tetraplets(&cp, 6)?;
        check_timestamp_tetraplets(&cp, 7)?;
        check_weight_peer_id(&cp.init_peer_id, &weight)?;
        let record = Record {
            key_id,
            value,
            peer_id: cp.host_id,
            set_by: cp.init_peer_id,
            relay_id,
            service_id,
            timestamp_created,
            signature,
            weight: weight.weight,
        };
        record.verify(current_timestamp_sec)?;

        let storage = get_storage()?;
        storage.check_key_existence(&record.key_id)?;
        storage.update_record(record.clone(), true)?;

        Ok(record)
    })
    .into()
}

/// Used for replication of host values to other nodes.
/// Similar to republish_values but with an additional check that value.set_by == init_peer_id
#[marine]
pub fn propagate_host_record(
    set_host_value: PutHostRecordResult,
    current_timestamp_sec: u64,
    weight: WeightResult,
) -> DhtResult {
    wrapped_try(|| {
        if !set_host_value.success || set_host_value.value.len() != 1 {
            return Err(ServiceError::InvalidSetHostValueResult);
        }

        let mut record = set_host_value.value[0].clone();
        record.verify(current_timestamp_sec)?;

        let call_parameters = marine_rs_sdk::get_call_parameters();
        check_host_value_tetraplets(&call_parameters, 0, &record)?;
        check_timestamp_tetraplets(&call_parameters, 1)?;
        check_weight_tetraplets(&call_parameters, 2)?;
        check_weight_peer_id(&record.peer_id, &weight)?;

        record.weight = weight.weight;
        let storage = get_storage()?;
        storage.check_key_existence(&record.key_id)?;
        storage.update_key_timestamp(&record.key_id, current_timestamp_sec)?;

        storage
            .merge_and_update_records(record.key_id.clone(), vec![record])
            .map(|_| ())
    })
    .into()
}

/// Return all values by key
#[marine]
pub fn get_records(key_id: String, current_timestamp_sec: u64) -> GetValuesResult {
    wrapped_try(|| {
        let call_parameters = marine_rs_sdk::get_call_parameters();
        check_timestamp_tetraplets(&call_parameters, 1)?;
        let storage = get_storage()?;
        storage.check_key_existence(&key_id)?;
        storage.update_key_timestamp(&key_id, current_timestamp_sec)?;
        storage.get_records(key_id)
    })
    .into()
}

/// If the key exists, then merge new records with existing (last-write-wins) and put
#[marine]
pub fn republish_records(
    records: Vec<Record>,
    weights: Vec<WeightResult>,
    current_timestamp_sec: u64,
) -> RepublishValuesResult {
    wrapped_try(|| {
        if records.is_empty() {
            return Ok(0);
        }

        let key_id = records[0].key_id.clone();
        let call_parameters = marine_rs_sdk::get_call_parameters();
        check_timestamp_tetraplets(&call_parameters, 2)?;

        for record in records.iter() {
            record.verify(current_timestamp_sec)?;
            if record.key_id != key_id {
                return Err(ServiceError::RecordsPublishingError);
            }
        }
        let storage = get_storage()?;
        storage.check_key_existence(&key_id)?;
        storage.update_key_timestamp(&key_id, current_timestamp_sec)?;
        storage.merge_and_update_records(key_id, records)
    })
    .into()
}

/// Remove host value by key and caller peer_id
#[marine]
pub fn clear_host_record(key_id: String, current_timestamp_sec: u64) -> DhtResult {
    wrapped_try(|| {
        let call_parameters = marine_rs_sdk::get_call_parameters();
        check_timestamp_tetraplets(&call_parameters, 1)?;
        let storage = get_storage()?;

        storage.check_key_existence(&key_id)?;
        storage.update_key_timestamp(&key_id, current_timestamp_sec)?;

        let peer_id = call_parameters.host_id;
        let set_by = call_parameters.init_peer_id;
        let deleted = storage.delete_record(key_id.clone(), peer_id, set_by)?;

        deleted.as_result((), ServiceError::HostValueNotFound(key_id))
    })
    .into()
}

#[marine]
pub fn merge_two(a: Vec<Record>, b: Vec<Record>) -> MergeResult {
    merge_records(a.into_iter().chain(b.into_iter()).collect()).into()
}
