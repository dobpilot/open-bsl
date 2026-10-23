# Add applicable catalog common attributes

## Why

Original demo reaches НаименованиеЯзык1 in the ГруппировкиЗадач object module. This is a configuration common attribute rather than an own attribute or global variable. The SDK now acquires its type and complete usage settings, and native probes establish new-object availability and conversion.

## What Changes

Retain a separate complete optional applied common-attribute collection in the ORM. Project acquired usage without reparsing descriptors, add its names to the existing receiver context, and implement the measured string attribute through each catalog object's own memory. Preserve unknown acquisition and keep other types explicitly unsupported until measured.
