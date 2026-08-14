.PHONY: aide donnees ingerer servir test verifier audit pedantic bench propre

SOURCE  ?= chevaux.jsonl
IMAGE   ?= data/equides.bin
ADRESSE ?= 127.0.0.1:8081
ADRESSE_ADMIN ?= 127.0.0.1:9091
LICENCE ?= Licence Ouverte / réutilisation d'informations publiques (CRPA art. L321-1)

aide:
	@echo "Cibles disponibles :"
	@echo "  make donnees     produit l'image binaire depuis $(SOURCE)"
	@echo "  make servir      lance l'API sur $(ADRESSE) (admin sur $(ADRESSE_ADMIN))"
	@echo "  make test        exécute la suite de tests"
	@echo "  make verifier    fmt + clippy + test (ce que la CI exécute)"
	@echo "  make audit       vulnérabilités connues et versions des dépendances"
	@echo "  make pedantic    lints écartés de la CI, pour relecture périodique"
	@echo "  make bench       banc de charge local (nécessite ab)"

donnees: ingerer

ingerer:
	@test -f "$(SOURCE)" || { echo "erreur : $(SOURCE) introuvable."; \
		echo "  Placez l'extraction JSONL à la racine, ou passez SOURCE=<chemin>."; exit 1; }
	cargo run --release --bin equides-ingest -- \
		--source "$(SOURCE)" \
		--sortie "$(IMAGE)" \
		--licence "$(LICENCE)"

servir:
	cargo run --release --bin equides-api -- \
		--image $(IMAGE) \
		--adresse $(ADRESSE) \
		--adresse-admin $(ADRESSE_ADMIN)

test:
	cargo test --locked

pedantic:
	cargo clippy --all-targets --locked -- -W clippy::pedantic

audit:
	cargo audit
	cargo update --dry-run --workspace

verifier:
	cargo fmt --check
	cargo clippy --all-targets --locked -- -D warnings
	cargo test --locked

bench:
	./scripts/bench.sh http://$(ADRESSE)


propre:
	cargo clean
	rm -f $(IMAGE)
