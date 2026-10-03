# 仅用于隔离 M1 契约验证；日志是测试证据，不能作为生产 hook 部署。
import json,os,sys,pathlib,time,sqlite3
p=pathlib.Path(__file__).parent
x=json.load(sys.stdin)
with (p/"hook-inputs.jsonl").open("a") as f:
 f.write(json.dumps({"input":x,"env":{k:v for k,v in os.environ.items() if k in ["CODEX_HOME","CODEX_THREAD_ID","CODEX_SESSION_ID","CODEX_VERSION"]},"time":time.time()})+"\n")
result={}
try:
 rows=[json.loads(l) for l in pathlib.Path(x["transcript_path"]).read_text().splitlines()]
 meta=next(r["payload"] for r in rows if r["type"]=="session_meta")
 if meta["id"]!=x["session_id"] or not isinstance(meta["source"],str): raise ValueError("not root")
 con=sqlite3.connect((pathlib.Path(os.environ["CODEX_HOME"])/"goals_1.sqlite").as_uri()+"?mode=ro",uri=True)
 goal=con.execute("select status from thread_goals where thread_id=?",(meta["id"],)).fetchone()
 stats=[r["payload"]["info"] for r in rows if r["type"]=="event_msg" and r["payload"].get("type")=="token_count"]
 if goal and goal[0]=="active" and stats:
  info=stats[-1];used=info["last_token_usage"]["total_tokens"];window=info["model_context_window"]
  if used*100>window*65:
   msg=f"M1_THRESHOLD_PROBE: 本地上下文估算 {used/window*100:.2f}%（{used}/{window}），严格超过65%。停止开始实质任务，先沿用已有 threshold-progress.md 保存已完成、未完成、验证、下一步及运行进程；仅当用户对当前 goal 有显式65%暂停请求时使用原生 update_goal 暂停并核对结果，否则确认并诚实报告未暂停。不能标complete或写数据库。然后回复交接，指引用户在同一目录自行新开对话，手动重建goal和暂停策略。"
   if x["hook_event_name"]=="PreToolUse":result={"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":msg}}
   elif not x["stop_hook_active"]:result={"decision":"block","reason":msg}
   else:result={"systemMessage":"隔离探针：仍active，请用户/goal pause。"}
except (KeyError,ValueError,OSError,sqlite3.Error,StopIteration): pass
print(json.dumps(result))
