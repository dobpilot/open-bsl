# Design

The isolated 1C 8.3.27.2342 probe measure-receiver-alias-collision.bsl returns 0 for both bare and explicit receiver calls, and 1 for the shadowing parameter. The same-named common-module export returns 999. Preserve the native oracle unchanged.

Add receiver_property(base) to the existing import_alias exclusions next to local and module variables. Existing expression resolution then uses the receiver property path. Do not introduce a second alias resolver, rewrite source, change global priorities or suppress missing methods on actual modules. The shared helper serves calls and reads consistently.

Original demo uses an owned tabular section, while the native probe uses a standard Structure-valued property. Both belong to the existing finite receiver context; test the demo-shaped section case independently without claiming tabular-section operations are already implemented.

Full workspace verification passed on 2026-10-02: formatting, Clippy, tests (355.1 seconds), rustdoc, facade consumer, normal core/VM dependency trees and all strict specifications. Log /tmp/open-bsl-receiver-alias-096-gates.log. Original demo startup passed the previous ФизическиеЛица.Сортировать compile barrier and failed at a missing Предопределенный receiver property after 62.5 seconds. The user script was not reached.
