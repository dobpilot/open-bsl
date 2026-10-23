# notification-subscriber-type Specification

## Purpose
Define the measured deliverable-notification subscriber enumeration without implying a notification delivery service.

## Requirements

### Requirement: Reached notification subscriber enum values
The runtime SHALL recognize ТипПодписчикаДоставляемыхУведомлений and DeliverableNotificationSubscriberType as aliases for the system enum containing the measured GCM, APNS and FCM members. Name matching SHALL follow the existing case-insensitive enum lookup behavior. Their string representations SHALL be GCM, APNs and FCM respectively, and their type string SHALL be DeliverableNotificationSubscriberType.

#### Scenario: Original dormant notification code compiles
- **WHEN** a module references ТипПодписчикаДоставляемыхУведомлений.GCM and ТипПодписчикаДоставляемыхУведомлений.APNS
- **THEN** both references resolve through the existing system enum registry

#### Scenario: Native values and aliases agree
- **WHEN** the retained notification-subscriber-type control runs
- **THEN** its value displays, type displays and alias comparisons match the native oracle

#### Scenario: Mixed case aliases resolve
- **WHEN** the program uses a differently cased Russian or English enum name and member
- **THEN** it receives the same enum value

### Requirement: Measured bare enum representation
The bare enum value and its type SHALL display as EnumDeliverableNotificationSubscriberType, distinct from the member type display. The Russian and English bare enum names SHALL compare equal.

#### Scenario: Bare names match native output
- **WHEN** the retained measure-bare control runs
- **THEN** its value display, type display and alias comparison match the native oracle
