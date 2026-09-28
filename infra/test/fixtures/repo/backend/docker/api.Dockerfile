# Test fixture: stands in for the real backend/docker/api.Dockerfile in unit tests.
FROM scratch
CMD ["dsa-api","serve"]
