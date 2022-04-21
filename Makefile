ROOT_DIR:=$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))

run-bench: ## Starts the benchmarking servers
	make -j 3 run-example-app run-varnish run-fragment-server

run-bench-app: ## Starts the example server for benchmarking
	cd bench/esi_bench_app && cargo run

run-varnish: ## Starts varnishd with the equivalent ESI example app
	/usr/local/opt/varnish/sbin/varnishd -n /usr/local/var/varnish -f $(ROOT_DIR)/bench/esi.vcl -s malloc,1G -a 127.0.0.1:8080 -F

run-fragment-server: ## Starts an HTTP server serving HTML pages with recursive ESI include tags
	cd esi_delay_tester && fastly compute serve --addr="127.0.0.1:8081"
