use base64::Engine;
use impeccable_browser::{cdp::Browser, BrowserEngine};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::capture::serve::{self, Server};

pub(super) struct Session {
    browser: Browser,
    server: Server,
}

impl Session {
    pub(super) fn new(env: &HashMap<String, String>, area: &Path) -> Result<Self, String> {
        let browser = BrowserEngine::new(env.clone())
            .launch()
            .map_err(|e| format!("浏览器不可用：{}", e.message))?;
        let server = serve::serve(area).map_err(|e| format!("视频服务器无法启动：{e}"))?;
        Ok(Self { browser, server })
    }

    pub(super) fn first_frame(&mut self, file: &str) -> Result<Vec<u8>, String> {
        let mut page = self.browser.new_isolated_page().map_err(|e| e.message)?;
        let result = (|| {
            page.goto(&self.server.url_for(file), "load", Duration::from_secs(30))
                .map_err(|e| format!("视频无法打开：{}", e.message))?;
            let result = page.evaluate_value(FIRST_FRAME).map_err(|e| e.message)?;
            if let Some(error) = result["error"].as_str() {
                return Err(format!("首帧未产出：{error}"));
            }
            let png = result["png"]
                .as_str()
                .and_then(|s| s.strip_prefix("data:image/png;base64,"))
                .ok_or("首帧未产出：浏览器未返回 PNG")?;
            base64::engine::general_purpose::STANDARD
                .decode(png)
                .map_err(|e| format!("首帧 PNG 无法读取：{e}"))
        })();
        page.close();
        result
    }
}

const FIRST_FRAME: &str = r#"(async () => {
  const src = location.href;
  document.documentElement.innerHTML = '<head></head><body></body>';
  const video = document.createElement('video');
  video.muted = true;
  video.preload = 'auto';
  document.body.append(video);
  const ready = await new Promise(resolve => {
    const timer = setTimeout(() => resolve({ error: '等待视频第 0 帧超时（20 秒）' }), 20000);
    video.addEventListener('loadeddata', () => { clearTimeout(timer); resolve(null); }, { once: true });
    video.addEventListener('error', () => {
      clearTimeout(timer);
      resolve({ error: '视频解码失败：' + video.error?.code + ' ' + video.error?.message });
    }, { once: true });
    video.src = src;
    video.load();
  });
  if (ready) return ready;
  video.pause();
  const canvas = document.createElement('canvas');
  canvas.width = video.videoWidth;
  canvas.height = video.videoHeight;
  canvas.getContext('2d').drawImage(video, 0, 0);
  return { width: video.videoWidth, height: video.videoHeight, png: canvas.toDataURL('image/png') };
})()"#;
