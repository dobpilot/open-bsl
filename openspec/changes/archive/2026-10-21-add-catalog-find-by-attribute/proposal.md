# Add catalog FindByAttribute

## Why
Original UNF Catalog.Контрагенты.ManagerModule calls НайтиПоРеквизиту("ИНН", value); the manager receiver lacks this member.

## What Changes
Measure and implement НайтиПоРеквизиту / FindByAttribute through the existing principal-bound query adapter. Validate acquired attribute names and return the concrete catalog reference, including the native empty reference on a genuine missing match. Preserve failures, rights, RLS and variable priority. No duplicate interpreter, direct SQL in ORM or fabricated successful result.
