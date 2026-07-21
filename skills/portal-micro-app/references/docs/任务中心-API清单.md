# 任务中心 app-task-center API 清单

---

## 一、pad-admin-api 接口（`VITE_AXIOS_BASE_URL`）

### 1. 认证 auth（`src/api/auth.ts`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| POST | `https://test01.supagent.cn/api/auth/login` | 登录 |
| POST | `https://test01.supagent.cn/api/auth/change-password` | 修改密码 |

### 2. 任务 task（`src/api/task.ts`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://test01.supagent.cn/api/v1/task/task-type` | 获取所有的任务类型 |
| GET | `https://test01.supagent.cn/api/v1/task/record/page` | 分页查询任务 |
| GET | `https://test01.supagent.cn/api/v1/task/record/{recordId}` | 获取任务记录详情 |
| GET | `https://test01.supagent.cn/api/v1/task/subject/statistics` | 统计各学科未完成任务数 |
| GET | `https://test01.supagent.cn/api/v1/task/type/statistics` | 统计各类型未完成任务数（支持按收藏过滤） |
| GET | `https://test01.supagent.cn/api/v1/task/liked/cnt` | 统计收藏任务数 |
| PUT | `https://test01.supagent.cn/api/v1/task/record` | 收藏/取消收藏任务、新任务已读 |
| POST | `https://test01.supagent.cn/api/v1/task/finish` | 完成任务 |
| POST | `https://test01.supagent.cn/api/v1/task/record-state` | 更新任务状态 |

### 3. 家庭成员 familyMember（`src/api/familyMember.ts`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://test01.supagent.cn/api/family-member/list` | 获取当前学生的家庭成员列表 |
| POST | `https://test01.supagent.cn/api/family-member` | 新增家庭成员 |
| PUT | `https://test01.supagent.cn/api/family-member/{id}` | 更新家庭成员信息 |
| DELETE | `https://test01.supagent.cn/api/family-member/{id}` | 删除家庭成员信息 |

### 4. 目标院校 targetSchool（`src/api/targetSchool.ts`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://test01.supagent.cn/api/target-school/page` | 分页获取目标院校信息 |
| GET | `https://test01.supagent.cn/api/target-school/{id}` | 获取院校详情 |
| POST | `https://test01.supagent.cn/api/target-school/choose` | 选择目标院校 |

### 5. 目标院校分数线 targetSchoolScoreLine（`src/api/targetSchoolScoreLine.ts`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://test01.supagent.cn/api/target-school-score-line/page` | 分页获取目标院校分数线信息 |

### 6. 学生中心 studentCenter（`src/api/studentCenter.ts`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://test01.supagent.cn/api/student-center/profile` | 获取当前学生详情 |

### 7. 系统字典 systemDict（`src/api/systemDict.ts`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://test01.supagent.cn/api/system-dict/all` | 获取全部字典列表 |

### 8. 试卷答题 exam-paper（`src/api/exam-paper.ts`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| POST | `https://test01.supagent.cn/api/exam-paper/answer-records` | 创建答题记录 |
| GET | `https://test01.supagent.cn/api/exam-paper/papers/{paperId}` | 获取试卷信息（前置页使用） |
| GET | `https://test01.supagent.cn/api/exam-paper/answer-records` | 获取历史答题记录列表 |
| GET | `https://test01.supagent.cn/api/resource/papers/{paperId}/detail` | 获取试卷详情（用于答题页） |
| GET | `https://test01.supagent.cn/api/resource/records/{recordId}/answers` | 获取答题记录信息（用于判断跳转） |
| POST | `https://test01.supagent.cn/api/resource/records/{recordId}/answers` | 保存答案（心跳接口） |
| POST | `https://test01.supagent.cn/api/resource/records/{recordId}/submit` | 提交答题（交卷） |
| GET | `https://test01.supagent.cn/api/resource/records/{recordId}/review-questions` | 获取自阅题目列表 |
| POST | `https://test01.supagent.cn/api/resource/records/{recordId}/review` | 提交自阅评分 |
| GET | `https://test01.supagent.cn/api/resource/records/{recordId}/report` | 获取答题报告 |

### 9. 错题本（`src/api/unitest.ts`，使用 `request`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| POST | `https://test01.supagent.cn/api/v1/task/wrongbook/add` | 添加错题到错题本 |
| GET | `https://test01.supagent.cn/api/v1/task/wrongbook/records` | 查询已加入错题本的题目列表 |

---

## 二、EES 接口（`VITE_ZY_BASE_URL`）

### 1. 阅读任务上报（`src/api/task.ts`，使用 `unitestRequest`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| POST | `https://pre-jz.zy.com/zyzxs/api/1/aipt/read_resource` | 阅读任务上报观看时长（taskId/resourceId/viewSecond/outStudentNo 走 query） |

### 2. 全日制学生听写/默写（`src/api/fulltimeStudent.ts`，使用 `studentRequest`）

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://pre-jz.zy.com/b/server/student/dictation/getTaskDetail` | 获取任务详情 |
| POST | `https://pre-jz.zy.com/b/server/student/dictation/correct` | AI 批阅（multipart/form-data） |
| POST | `https://pre-jz.zy.com/b/server/student/dictation/uploadMarkImage` | 上传批阅图片（multipart/form-data） |
| GET | `https://pre-jz.zy.com/b/server/student/dictation/listStudentErrorWord` | 获取学生错词本（单词本） |
| POST | `https://pre-jz.zy.com/b/server/student/dictation/feedback` | 意见反馈 |
| GET | `https://pre-jz.zy.com/b/server/student/dictation/getFeedbackCategory` | 获取反馈分类 |

### 3. unitest 考试服务（`src/api/unitest.ts`，使用 `unitestRequest`）

#### 考试相关

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://pre-jz.zy.com/unitest-server/front/getExam` | 获取考试信息 |
| POST | `https://pre-jz.zy.com/unitest-server/front/examLogin` | 考试登录 |
| POST | `https://pre-jz.zy.com/unitest-server/front/startExam` | 开始考试 |
| POST | `https://pre-jz.zy.com/unitest-server/front/finishExam` | 结束考试（交卷，含手动/倒计时/切屏自动） |
| POST | `https://pre-jz.zy.com/unitest-server/front/heartbeat` | 心跳同步（答案/事件/监控图片上传） |
| GET | `https://pre-jz.zy.com/unitest-server/front/getPaperDetails` | 获取试卷详情 |
| GET | `https://pre-jz.zy.com/unitest-server/front/getNowTime` | 获取系统当前时间 |
| GET | `https://pre-jz.zy.com/unitest-server/front/getStuExamAnswer` | 获取学生考试答案（⚠️ 枚举已定义、代码中未调用） |

#### 报告相关

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://pre-jz.zy.com/unitest-server/front/getStuExamReport` | 获取学生考试报告 |
| GET | `https://pre-jz.zy.com/unitest-server/front/getReportInfo` | 获取报告信息 |
| GET | `https://pre-jz.zy.com/unitest-server/front/getStudentAnswerStatus` | 查询学生答题状态 |

#### 学生相关

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://pre-jz.zy.com/unitest-server/front/findStudentByMemberId` | 根据 memberId 获取学生 |

#### 批阅相关

| 请求方式 | 完整请求地址 | 接口用途 |
| --- | --- | --- |
| GET | `https://pre-jz.zy.com/unitest-server/manage/paperCorrect/getStudentAnswerByMemberId` | 获取需要批阅的学生题目和答题信息 |
| POST | `https://pre-jz.zy.com/unitest-server/manage/paperCorrect/saveQuestionsCorrect` | 提交批阅结果（自阅/互阅） |

---

## 汇总

| 分类 | baseURL | 请求实例 | 接口数量 |
| --- | --- | --- | --- |
| pad-admin-api | `https://test01.supagent.cn` | `request` | 33 |
| EES | `https://pre-jz.zy.com` | `unitestRequest` / `studentRequest` | 21（含 1 个已定义未调用） |
