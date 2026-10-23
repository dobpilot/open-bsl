# Why
Original BUH compilation reaches QueryRecordType.TotalByHierarchy in the StatisticsIndicators catalog manager.

# What Changes
Declare the four native query record types using the existing system enum and type tables, both aliases, localized type lookup and measured presentations.

# Impact
System enum declaration only. No query selection RecordType method, grouped traversal, query execution or new dependencies.
