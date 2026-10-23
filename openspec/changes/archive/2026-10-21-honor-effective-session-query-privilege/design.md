# Design
The original register manager source enables safe-mode disabling and privileged mode immediately before Query.Execute and disables both afterwards. The core already exposes effective frame mode through CallContext::privileged_mode, including suppression; the adapter must use that result, not a second session or thread-local flag.

Keep restricted query preparation and metadata validation. When effective mode permits privileged reads, provide explicit unrestricted decisions for the prepared targets and bypass ordinary role field checks. When mode is false, use the existing user checks, RLS and lazy session parameter path exactly as before. Privileged functional-option source validation remains its existing independently bounded path.

A native own-base control of ordinary refusal, privileged success, mode reset and safe-mode suppression precedes implementation. Do not infer this contract from a successful open-bsl run.
