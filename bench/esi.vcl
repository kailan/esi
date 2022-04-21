vcl 4.1;

backend delay_tester {
    .host = "127.0.0.1";
    .port = "8081";
}

sub vcl_backend_response {
    set beresp.do_esi = true;
}
