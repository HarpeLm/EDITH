# Instance SearXNG locale d'Edith

docker run -d --name searxng -p 8888:8080 \
  -v $(pwd)/settings.yml:/etc/searxng/settings.yml searxng/searxng

Le format JSON est activé dans settings.yml (nécessaire pour l'outil search_web).
