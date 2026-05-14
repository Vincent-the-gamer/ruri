---
layout: doc
title: "Proxy Configuration"
lastUpdated: true
---

# Proxy Configuration

Proxy Configuration allows you to route outgoing requests through a proxy server. This is useful when you're behind a corporate firewall, need to use a specific proxy for API calls, or want to control network traffic.

## Why Use a Proxy?

- **Corporate network requirements** — Route traffic through your company's proxy server
- **API access** — Use a proxy to access APIs that are blocked or restricted in your region
- **Traffic control** — Monitor and control outgoing requests
- **Anonymity** — Hide your IP address when making API calls

## Proxy Settings

Proxy settings can be configured per [Config Profile](/config-profiles) or in the Chat Configuration dialog.

### Basic Settings

| Setting | Description |
| ------- | ----------- |
| **Enable Proxy** | Toggle proxy on/off |
| **Proxy URL** | The proxy server address (e.g., `http://127.0.0.1:7890`) |
| **Proxy Mode** | Global or Rules mode |
| **Bypass Localhost** | Skip proxy for localhost and 127.0.0.1 |

### Proxy URL Formats

Ruri supports multiple proxy protocols:

| Protocol | Example | Description |
| -------- | ------- | ----------- |
| HTTP | `http://127.0.0.1:7890` | Standard HTTP proxy |
| HTTPS | `https://127.0.0.1:7890` | HTTPS proxy |
| SOCKS5 | `socks5://127.0.0.1:1080` | SOCKS5 proxy |
| SOCKS4 | `socks4://127.0.0.1:1080` | SOCKS4 proxy |

### Authentication

If your proxy requires authentication, include credentials in the URL:

```
http://username:password@127.0.0.1:7890
```

Or use the dedicated username/password fields in the UI.

## Proxy Modes

### Global Mode

In Global mode, all outgoing requests go through the proxy server. This is the simplest setup — just enable the proxy and all traffic is routed.

**Best for:**
- Simple setups where all traffic should go through the proxy
- Corporate environments with strict network policies

### Rules Mode

Rules Mode allows you to define Clash-style rules to control which traffic goes through the proxy and which goes direct. Rules are matched in order, and the first match wins.

**Best for:**
- Selective proxying (e.g., only proxy API calls, not local traffic)
- Advanced users who need fine-grained control

#### Supported Rule Types

| Rule Type | Description | Example |
| --------- | ----------- | ------- |
| `DOMAIN` | Match exact domain | `DOMAIN,api.openai.com` |
| `DOMAIN-SUFFIX` | Match domain suffix | `DOMAIN-SUFFIX,.github.com` |
| `DOMAIN-KEYWORD` | Match domain keyword | `DOMAIN-KEYWORD,google` |
| `IP-CIDR` | Match IP range | `IP-CIDR,192.168.0.0/16` |
| `GEOIP` | Match country code | `GEOIP,CN` |
| `MATCH` | Match all remaining traffic | `MATCH,DIRECT` |

#### Example Rules

```
DOMAIN-SUFFIX,.openai.com,PROXY
DOMAIN-SUFFIX,.anthropic.com,PROXY
DOMAIN-SUFFIX,.github.com,PROXY
GEOIP,CN,DIRECT
MATCH,DIRECT
```

This configuration proxies requests to OpenAI, Anthropic, and GitHub, while all Chinese IP addresses and remaining traffic go direct.

## Bypass Settings

### Bypass Localhost

When enabled, requests to `localhost` and `127.0.0.1` skip the proxy. This is useful when:

- Running local services that shouldn't go through the proxy
- Testing with local development servers

### Bypass Domains

You can specify domains that should always bypass the proxy:

1. Enter a domain pattern (e.g., `*.local`, `*.internal`)
2. Press Enter to add
3. These domains will always use direct connections

## Configuring Proxy in Config Profiles

Proxy settings can be included in your [Config Profiles](/config-profiles) for easy switching:

1. Go to **Config Profiles**
2. Create or edit a profile
3. Scroll to the **Proxy Configuration** section
4. Configure your proxy settings
5. Save the profile

This lets you have:

- A **Work** profile with your corporate proxy
- A **Personal** profile with no proxy or a different proxy
- Quick switching between them

## Troubleshooting

### Proxy Connection Fails

- Verify the proxy URL is correct
- Check that the proxy server is running and accessible
- Ensure the proxy protocol is supported (HTTP, HTTPS, SOCKS4, SOCKS5)
- Check firewall settings that might block the connection

### API Calls Time Out

- The proxy server might be slow or overloaded
- Try reducing the timeout or switching to a different proxy
- Check if the proxy blocks the target API endpoint

### Rules Not Working as Expected

- Rules are matched in order — make sure specific rules come before general ones
- Use `MATCH,DIRECT` or `MATCH,PROXY` as the last rule to catch all remaining traffic
- Check for typos in domain names or IP ranges

## Tips

- **Test with Global mode first** — Verify your proxy works before adding complex rules
- **Keep rules simple** — Start with a few essential rules and add more as needed
- **Use Bypass Localhost** — Always enable this if you use local services
- **Monitor proxy usage** — Some proxies have bandwidth or request limits
- **Document your rules** — Keep notes on why you added specific rules for future reference