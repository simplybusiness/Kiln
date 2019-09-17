This service has a single endpoint, exposed behind an AWS API gateway.

Requests to this endpoint must be in JSON format in the following format:
```
{
        application_name: String,
        git_branch: String,
        git_commit_hash: String,
        tool_name: String,
        tool_output: String,
        output_format: String(Json|PlainText),
        start_time: Timestamp string in RFC3339 format,
        end_time: Timestamp string in RFC3339 format,
        environment: String(Local|CI),
        tool_version: String (optional)
}
```

(Where the above conflicts with the validation logic in kiln_lib, the validation logic and corresponding tests are correct, and creating an issue to correct this would be appreciated!)

All fields not explicitly marked optional are required. If a field is present, it must not be empty. A successful request will return an HTTP 200 OK with an empty body.

A request that fails validation will return an HTTP 400 Bad Request with a JSON body containing an "error_code" field and an "error_message" field with a brief description of the reason the request body didn't pass validation.
