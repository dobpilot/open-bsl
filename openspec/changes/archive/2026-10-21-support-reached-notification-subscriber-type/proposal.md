# Support reached notification subscriber enum values

## Why
Original buh cannot compile the manager module of НастройкиОтправкиPushУведомлений because the existing system enum registry lacks ТипПодписчикаДоставляемыхУведомлений. Its dormant code references GCM and APNS.

## What Changes
Add the native-measured GCM, APNS and FCM values and their English namespace alias to the existing base runtime enum table. Preserve measured display strings and type names.

## Impact
This adds enum names, not a notification transport or a separate name resolver. Existing resolution priorities and dependencies remain unchanged.
