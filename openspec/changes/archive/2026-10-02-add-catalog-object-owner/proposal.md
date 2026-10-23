# Add acquired catalog object owner

Original buh startup reaches the standard Owner attribute in a catalog object module. The SDK already acquires catalog owner GUIDs, but the ORM does not retain them and its object receiver does not expose Owner. Preserve this acquired list and implement the native new-object behavior without altering global resolution or accessing the database.
