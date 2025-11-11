# Swagger UI Documentation

This project includes interactive API documentation using Swagger UI powered by `utoipa`.

## Accessing Swagger UI

Once the server is running, access the Swagger UI at:
```
http://localhost:3000/swagger-ui
```

The OpenAPI 3.0 specification JSON is available at:
```
http://localhost:3000/api-docs/openapi.json
```

## Features

### Interactive API Explorer
- **Browse all endpoints** organized by tags (Authentication, Users)
- **View detailed schemas** for request/response bodies
- **See validation rules** and field constraints
- **Test endpoints** directly from the browser
- **JWT authentication** integrated into the UI

### API Tags

#### Authentication
- `POST /api/auth/register` - Register a new user
- `POST /api/auth/login` - Login and receive JWT token

#### Users
- `GET /api/users` - Get all users (protected)
- `POST /api/users` - Create a user
- `GET /api/users/profile` - Get current user profile (protected)

## Using Swagger UI

### Testing Public Endpoints

1. Open Swagger UI in your browser
2. Navigate to the **Authentication** section
3. Click on `POST /api/auth/register`
4. Click "Try it out"
5. Edit the request body with your data
6. Click "Execute"
7. View the response below

### Testing Protected Endpoints

Protected endpoints require JWT authentication:

1. **First, login to get a token:**
   - Use `POST /api/auth/login` endpoint
   - Execute the request
   - Copy the `access_token` from the response

2. **Authorize Swagger UI:**
   - Click the 🔒 **Authorize** button at the top
   - Enter: `Bearer YOUR_ACCESS_TOKEN`
   - Click "Authorize"
   - Click "Close"

3. **Test protected endpoints:**
   - All protected endpoints will now include your token
   - Try `GET /api/users/profile`
   - The token is automatically added to requests

### Example Workflow

```bash
# 1. Register a user
POST /api/auth/register
{
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "password": "password123"
}

# 2. Login to get token
POST /api/auth/login
{
  "email": "john@example.com",
  "password": "password123"
}
# Response includes: { "access_token": "eyJ0eXAi...", ... }

# 3. Click 🔒 Authorize button
# Enter: Bearer eyJ0eXAi...

# 4. Test protected endpoint
GET /api/users/profile
# Automatically includes Authorization header
```

## Schema Documentation

Swagger UI automatically displays:

- **Required fields** marked with *
- **Field types** (string, integer, uuid, etc.)
- **Validation rules** (email format, minimum length, etc.)
- **Example values** for easier testing
- **Nested objects** with expandable views

## Implementation Details

### Adding Documentation to New Endpoints

To document a new endpoint, add the `#[utoipa::path]` macro:

```rust
/// Get user by ID
#[utoipa::path(
    get,
    path = "/api/users/{id}",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User found", body = User),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
pub async fn get_user(
    Path(id): Path<Uuid>,
    auth_user: AuthUser,
) -> Result<Json<User>, AppError> {
    // implementation
}
```

### Adding Schemas

For request/response models, add `#[derive(ToSchema)]`:

```rust
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserDto {
    #[schema(example = "john@example.com")]
    pub email: String,
    
    #[schema(example = "John")]
    pub first_name: String,
}
```

### Registering in OpenAPI Spec

Update `src/docs.rs`:

```rust
#[derive(OpenApi)]
#[openapi(
    paths(
        // Add your endpoint here
        crate::modules::users::controller::get_user,
    ),
    components(
        schemas(
            // Add your schema here
            UserDto,
        )
    ),
    // ... rest of config
)]
pub struct ApiDoc;
```

## Security Configuration

JWT Bearer authentication is configured in the OpenAPI spec:

```rust
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            )
        }
    }
}
```

Endpoints requiring authentication specify:
```rust
security(
    ("bearer_auth" = [])
)
```

## Customization

### Change Swagger UI Path

Edit `src/router.rs`:
```rust
.merge(SwaggerUi::new("/docs").url("/api-spec/openapi.json", ApiDoc::openapi()))
```

### Add API Metadata

Edit `src/docs.rs`:
```rust
info(
    title = "Your API Name",
    version = "1.0.0",
    description = "Your API description",
    contact(
        name = "Support Team",
        email = "support@example.com"
    ),
    license(
        name = "MIT"
    )
)
```

### Add More Tags

```rust
tags(
    (name = "Authentication", description = "User authentication endpoints"),
    (name = "Users", description = "User management"),
    (name = "Posts", description = "Blog posts management"),
)
```

## Benefits

1. **Self-documenting API** - Code and docs stay in sync
2. **Interactive testing** - No need for external tools like Postman
3. **Type-safe** - Generated from Rust types at compile time
4. **Standards-compliant** - Uses OpenAPI 3.0 specification
5. **Developer-friendly** - Easy to explore and understand the API
6. **Client generation** - OpenAPI spec can generate clients in any language

## Alternative Documentation UIs

While this project uses Swagger UI, you can easily swap it for alternatives:

### RapiDoc
```toml
utoipa-rapidoc = "6.0"
```

### ReDoc
```toml
utoipa-redoc = "6.0"
```

### Scalar
```toml
utoipa-scalar = "0.3"
```

## Troubleshooting

### Swagger UI not loading
- Ensure server is running on port 3000
- Check browser console for errors
- Verify `/api-docs/openapi.json` returns valid JSON

### Endpoints not appearing
- Ensure endpoint is registered in `src/docs.rs`
- Check that `#[utoipa::path]` macro is present
- Verify the path matches your route

### Schema not rendering
- Add `#[derive(ToSchema)]` to your struct
- Register schema in `components(schemas(...))`
- Ensure all nested types also derive `ToSchema`

### Authorization not working
- Verify JWT token is valid
- Check token format: `Bearer <token>`
- Ensure protected endpoints have `security(("bearer_auth" = []))`

## Resources

- [utoipa documentation](https://docs.rs/utoipa/)
- [OpenAPI 3.0 Specification](https://swagger.io/specification/)
- [Swagger UI documentation](https://swagger.io/tools/swagger-ui/)
