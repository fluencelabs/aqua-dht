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

use crate::error::ServiceError;
use crate::misc::extract_public_key;
use fluence_keypair::Signature;
use marine_rs_sdk::marine;
use sha2::{Digest, Sha256};

#[marine]
#[derive(Default, Clone)]
pub struct Key {
    pub key_id: String,
    pub key: String,
    pub peer_id: String,
    pub timestamp_created: u64,
    pub signature: Vec<u8>,
    pub timestamp_published: u64,
    pub pinned: bool,
    pub weight: u32,
}

impl Key {
    pub fn new(
        key: String,
        peer_id: String,
        timestamp_created: u64,
        signature: Vec<u8>,
        timestamp_published: u64,
        pinned: bool,
        weight: u32,
    ) -> Self {
        let key_id = Self::get_key_id(&key, &peer_id);

        Self {
            key_id,
            key,
            peer_id,
            timestamp_created,
            signature,
            timestamp_published,
            pinned,
            weight,
        }
    }

    pub fn get_key_id(key: &str, peer_id: &str) -> String {
        format!("{}{}", key, peer_id)
    }

    pub fn signature_bytes(key: String, peer_id: String, timestamp_created: u64) -> Vec<u8> {
        let mut metadata = Vec::new();
        metadata.extend(key.as_bytes());
        metadata.extend(peer_id.as_bytes());
        metadata.extend(timestamp_created.to_le_bytes());

        let mut hasher = Sha256::new();
        hasher.update(metadata);
        hasher.finalize().to_vec()
    }

    pub fn verify(&self, current_timestamp_sec: u64) -> Result<(), ServiceError> {
        if self.timestamp_created > current_timestamp_sec {
            return Err(ServiceError::InvalidKeyTimestamp);
        }

        self.verify_signature()
    }

    pub fn verify_signature(&self) -> Result<(), ServiceError> {
        let pk = extract_public_key(self.peer_id.clone())?;
        let bytes = &Self::signature_bytes(
            self.key.clone(),
            self.peer_id.clone(),
            self.timestamp_created,
        );
        let signature = Signature::from_bytes(pk.get_key_format(), self.signature.clone());
        pk.verify(&bytes, &signature)
            .map_err(|e| ServiceError::InvalidKeySignature(self.key.clone(), e))
    }
}
