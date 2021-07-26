/**
 *
 * This file is auto-generated. Do not edit manually: changes may be erased.
 * Generated by Aqua compiler: https://github.com/fluencelabs/aqua/. 
 * If you find any bugs, please write an issue on GitHub: https://github.com/fluencelabs/aqua/issues
 * Aqua version: 0.1.10-167
 *
 */
import { FluenceClient, PeerIdB58 } from '@fluencelabs/fluence';
import { RequestFlowBuilder } from '@fluencelabs/fluence/dist/api.unstable';
import { RequestFlow } from '@fluencelabs/fluence/dist/internal/RequestFlow';



export async function clearExpired(client: FluenceClient, config?: {ttl?: number}): Promise<void> {
    let request: RequestFlow;
    const promise = new Promise<void>((resolve, reject) => {
        const r = new RequestFlowBuilder()
            .disableInjections()
            .withRawScript(
                `
(xor
 (seq
  (seq
   (call %init_peer_id% ("getDataSrv" "-relay-") [] -relay-)
   (call %init_peer_id% ("peer" "timestamp_sec") [] t)
  )
  (call %init_peer_id% ("aqua-dht" "clear_expired") [t])
 )
 (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 1])
)

            `,
            )
            .configHandler((h) => {
                h.on('getDataSrv', '-relay-', () => {
                    return client.relayPeerId!;
                });
                
                
                h.onEvent('errorHandlingSrv', 'error', (args) => {
                    // assuming error is the single argument
                    const [err] = args;
                    reject(err);
                });
            })
            .handleScriptError(reject)
            .handleTimeout(() => {
                reject('Request timed out for clearExpired');
            })
        if(config && config.ttl) {
            r.withTTL(config.ttl)
        }
        request = r.build();
    });
    await client.initiateFlow(request!);
    return Promise.race([promise, Promise.resolve()]);
}
      


export async function replicate(client: FluenceClient, config?: {ttl?: number}): Promise<void> {
    let request: RequestFlow;
    const promise = new Promise<void>((resolve, reject) => {
        const r = new RequestFlowBuilder()
            .disableInjections()
            .withRawScript(
                `
(xor
 (seq
  (seq
   (seq
    (call %init_peer_id% ("getDataSrv" "-relay-") [] -relay-)
    (call %init_peer_id% ("peer" "timestamp_sec") [] t)
   )
   (call %init_peer_id% ("aqua-dht" "evict_stale") [t] res)
  )
  (fold res.$.results! r
   (par
    (seq
     (seq
      (call %init_peer_id% ("op" "string_to_b58") [r.$.key.key!] k)
      (call %init_peer_id% ("kad" "neighborhood") [k $nil $nil] nodes)
     )
     (fold nodes n
      (par
       (seq
        (call -relay- ("op" "noop") [])
        (xor
         (seq
          (call n ("aqua-dht" "republish_key") [r.$.key! t])
          (call n ("aqua-dht" "republish_values") [r.$.key.key! r.$.records! t])
         )
         (seq
          (call -relay- ("op" "noop") [])
          (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 1])
         )
        )
       )
       (next n)
      )
     )
    )
    (next r)
   )
  )
 )
 (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 2])
)

            `,
            )
            .configHandler((h) => {
                h.on('getDataSrv', '-relay-', () => {
                    return client.relayPeerId!;
                });
                
                
                h.onEvent('errorHandlingSrv', 'error', (args) => {
                    // assuming error is the single argument
                    const [err] = args;
                    reject(err);
                });
            })
            .handleScriptError(reject)
            .handleTimeout(() => {
                reject('Request timed out for replicate');
            })
        if(config && config.ttl) {
            r.withTTL(config.ttl)
        }
        request = r.build();
    });
    await client.initiateFlow(request!);
    return Promise.race([promise, Promise.resolve()]);
}
      