// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { sanitizeHtml } from './utils';

describe('sanitizeHtml (XSS)', () => {
  it('strips script tags and event handlers', () => {
    const out = sanitizeHtml('<p>hi</p><script>alert(1)</script><img src=x onerror=alert(1)>');
    expect(out).not.toContain('<script');
    expect(out).not.toContain('onerror');
    expect(out).toContain('<p>hi</p>');
  });

  it('strips quoted and unquoted on* attributes', () => {
    expect(sanitizeHtml('<a href="#" onclick="x()">a</a>')).not.toContain('onclick');
    expect(sanitizeHtml('<img src="x" onload=alert(1)>')).not.toContain('onload');
    expect(sanitizeHtml('<svg onload=alert(1)></svg>')).not.toContain('onload');
  });

  it('blocks javascript: URLs', () => {
    expect(sanitizeHtml('<a href="javascript:alert(1)">x</a>')).not.toContain('javascript:');
  });

  it('allows formatting and links', () => {
    const out = sanitizeHtml('<strong>bold</strong> <a href="https://x.com">link</a> <code>c</code>');
    expect(out).toContain('<strong>bold</strong>');
    expect(out).toContain('<a href="https://x.com">link</a>');
    expect(out).toContain('<code>c</code>');
  });

  it('forbids iframes and form controls', () => {
    expect(sanitizeHtml('<iframe src="https://evil.com"></iframe>')).not.toContain('<iframe');
    expect(sanitizeHtml('<form action="https://evil.com"><input name="x"></form>')).not.toContain('<form');
    expect(sanitizeHtml('<input type="text">')).not.toContain('<input');
  });
});
