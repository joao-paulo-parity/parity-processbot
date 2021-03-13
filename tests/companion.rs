use httptest::{mappers::*, responders::*, Expectation, Server};
use serde_json::json;

#[tokio::test]
async fn case1() {
	env_logger::init();
	let github_api = Server::run();
	let mut request_count: usize = 0;
	server.expect(
		Expectation::matching(request::method_path("GET", "/foo"))
			.respond_with(|| {
				request_count += 1;
				if request_count == 1 {
					status_code(405)()
				} else {
					status_code(201)()
				}
			}),
	);

	// The server provides server.addr() that returns the address of the
	// locally running server, or more conveniently provides a server.url()
	// method that gives a fully formed http url to the provided path.
	let url = server.url("/foo");

	// Now test your http client against the server.
	let client = hyper::Client::new();
	// Issue the GET /foo to the server.
	let resp = client.get(url).await.unwrap();
	// Optionally use response matchers to assert the server responded as
	// expected.

	// Assert the response was a 200.
	assert_eq!(200, resp.status().as_u16());

	// Issue a POST /bar with {'foo': 'bar'} json body.
	let post_req = http::Request::post(server.url("/bar"))
		.body(json!({"foo": "bar"}).to_string().into())
		.unwrap();
	// Read the entire response body into a Vec<u8> to allow using the body
	// response matcher.
	let resp = read_response_body(client.request(post_req)).await;
	// Assert the response was a 200 with a json body of {'result': 'success'}
	assert_eq!(200, resp.status().as_u16());
	assert_eq!(
		json!({"result": "success"}),
		serde_json::from_slice::<serde_json::Value>(resp.body()).unwrap()
	);
}
