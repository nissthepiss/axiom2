use tonic::transport::Channel;
use tonic_reflection::pb::v1alpha::server_reflection_client::ServerReflectionClient as ReflectionClient;
use tonic_reflection::pb::v1alpha::server_reflection_request::Message as RequestMessage;
use tonic_reflection::pb::v1alpha::{ServerReflectionRequest, ServerReflectionResponse};
use tonic_reflection::pb::v1alpha::server_reflection_response::Message as ResponseMessage;
use prost::Message as ProstMessage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = "https://yellowstone-solana-mainnet.core.chainstack.com:443";

    println!("Connecting to: {}", endpoint);
    println!("Discovering gRPC services via reflection...");
    println!();

    // Create channel with TLS
    let channel = Channel::from_shared(endpoint.to_string())?
        .timeout(std::time::Duration::from_secs(30))
        .tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;

    println!("Connected successfully");
    println!();

    // Create reflection client
    let mut client = ReflectionClient::new(channel);

    // List all services
    println!("Requesting service list...");
    let request = ServerReflectionRequest {
        host: String::new(),
        message: Some(RequestMessage::ListServices(String::new())),
    };

    match client.server_reflection_info(request).await {
        Ok(response) => {
            let mut stream: tonic::codec::Streaming<ServerReflectionResponse> = response.into_inner();

            // Process all messages in the stream
            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => {
                        if let Some(ResponseMessage::ListServicesResponse(list_response)) = msg.message {
                            println!("Found {} services:", list_response.services.len());
                            println!();

                            for service in list_response.services {
                                println!("  - {}", service.name);
                            }

                            // Now get file descriptor for the first service to see its methods
                            if let Some(first_service) = list_response.services.first() {
                                println!();
                                println!("Getting file descriptor for: {}", first_service.name);
                                println!();

                                let file_request = ServerReflectionRequest {
                                    host: String::new(),
                                    message: Some(RequestMessage::FileByFilename(first_service.name.clone())),
                                };

                                // Need a new client for the second request
                                let channel2 = Channel::from_shared(endpoint.to_string())?
                                    .timeout(std::time::Duration::from_secs(30))
                                    .tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())?
                                    .connect()
                                    .await?;

                                let mut client2 = ReflectionClient::new(channel2);

                                if let Ok(file_response) = client2.server_reflection_info(file_request).await {
                                    let mut file_stream: tonic::codec::Streaming<ServerReflectionResponse> = file_response.into_inner();

                                    while let Some(file_result) = file_stream.next().await {
                                        match file_result {
                                            Ok(file_msg) => {
                                                println!("File descriptor response received");
                                                if let Some(ResponseMessage::FileDescriptorResponse(fd_response)) = file_msg.message {
                                                    for fd in &fd_response.file_descriptor_proto {
                                                        // Decode the file descriptor
                                                        if let Ok(descriptor) = prost_types::FileDescriptorProto::decode(&*fd) {
                                                            println!("Package: {}", descriptor.package);
                                                            println!("Name: {}", descriptor.name);

                                                            for service in &descriptor.service {
                                                                println!();
                                                                println!("  Service: {}", service.name);
                                                                for method in &service.method {
                                                                    println!("    - {} (input: {}, output: {})",
                                                                        method.name,
                                                                        method.input_type,
                                                                        method.output_type);

                                                                    if method.client_streaming && method.server_streaming {
                                                                        println!("      -> Bidirectional streaming");
                                                                    } else if method.client_streaming {
                                                                        println!("      -> Client streaming");
                                                                    } else if method.server_streaming {
                                                                        println!("      -> Server streaming");
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Error receiving file descriptor: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving message: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            let code = e.code();
            let message = e.message();
            eprintln!("Error listing services:");
            eprintln!("  Status: {:?}", code);
            eprintln!("  Message: {}", message);

            // Try alternative approach
            eprintln!();
            eprintln!("The endpoint may not have reflection enabled.");
            eprintln!("Let's try to connect directly to test the connection...");
        }
    }

    Ok(())
}
