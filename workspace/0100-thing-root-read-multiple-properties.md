# TD-driven Thing-root ReadMultipleProperties

Status: MIGRATED

TD 1.1 §5.3.1.1 defines the default request as an array of Property names,
serialized with the selected Form's content type. §6.3.9.6 requires readable
Properties and distinguishes all from selected scope. The existing finite
owners suffice; neither individual-read fan-out nor a new collection runtime
preserves the intended native interaction more simply.

Retain the full readable name/schema set at preparation, using the existing
bounded semantic closure and independently paid temporary/retained owners.
Keep each invocation's set separate as a fixed bitmap into that original schema.
The actual captured data service encodes the default name array and validates
exact selected results. Producer output validation receives the original input
alongside the original candidate; re-reading global/current selection would
misattribute late results or concurrent calls.

Use an explicitly described HTTP POST/200 JSON Form for the native slice.
GET with a body has no generally defined request semantics; inventing a fixed
query parameter would turn a Binding convention into shared Engine meaning.
TD URI templates are an alternative mapping, not the default array encoding.
This slice declines templates/uriVariables rather than interpreting them as a
selection or claiming URI expansion support. HTTP owns method/path matching,
framing, encoding and whole-result refusal; aggregation remains application
code. Empty selection is supported as [] -> {}. No snapshot is promised.

Affected owners: TD loans, Planning, Core collection/data contracts, Binding
SPI, runtime safety, API ownership, WP-200/300/400/600 and the HTTP README.
ReadAllProperties retains its complete readable-set validation. Selected writes,
continuing collections, general schemas/security and production qualification
remain outside the claim. Exact implementation evidence belongs to tests and
the review PR; this investigation grants no independent acceptance.
